use anyhow::Error;
use http::{Request, Response};
use if_chain::if_chain;
use yew::services::fetch::{FetchService, FetchTask};
use yew::{
    format::{Json, Nothing},
    html::Html,
    prelude::*,
    services::Task,
    Properties,
};
use dotenv_codegen::dotenv;

use std::{collections::HashMap, rc::Rc};

use crate::app::SpotifyUser;
use crate::spotify_types::SpotifyUserProfile;

pub struct Connect {
    link: ComponentLink<Self>,
    props: Props,
    fetch_task: Option<FetchTask>,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub spotify_user: Option<Rc<SpotifyUser>>,
    pub onconnect: Callback<SpotifyUser>,
}

pub enum Msg {
    UserProfileLoaded(String, String, i64),
    Noop,
}

impl Component for Connect {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut connect = Connect {
            link,
            props,
            fetch_task: None,
        };

        let window = web_sys::window().expect("window not available");

        let hash = window
            .location()
            .hash()
            .expect("location hash not available");
        let hash_params = parse_query(&hash);

        if_chain! {
            if let Some(Some(access_token)) = hash_params.get("access_token");
            if let Some(Some(expires_in)) = hash_params.get("expires_in");
            then {
                let now = js_sys::Date::now() as i64;
                let expires_in = expires_in.parse::<i64>().expect("parse expires_in");
                let expiration_ts = now + expires_in * 1_000;
                connect.get_user_profile(access_token.clone(), expiration_ts);
                window.location().set_hash("").expect("set location hash");
            }
        }

        connect
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::UserProfileLoaded(user_id, access_token, expiration_ts) => {
                self.props.onconnect.emit(SpotifyUser {
                    user_id,
                    access_token,
                    expiration_ts,
                });
            }
            Msg::Noop => {}
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        let window = web_sys::window().expect("window not available");

        let search = window
            .location()
            .search()
            .expect("location search not available");
        let search_params = parse_query(&search);

        let render_error_message = if let Some(Some(error)) = search_params.get("error") {
            html! {<div class="error">{error}</div>}
        } else {
            html! {}
        };

        let client_id = dotenv!("CLIENT_ID");
        let redirect_uri = "http://localhost:8000";
        let scopes = vec![
            "playlist-read-private",
            "playlist-modify-private",
            "user-library-read",
            "user-library-modify",
        ]
        .join(" ");
        let url = format!("https://accounts.spotify.com/authorize?client_id={}&response_type=token&redirect_uri={}&scope={}", client_id, redirect_uri, scopes);
        html! {
            <div>
                {render_error_message}
                {
                    if self.fetch_task.as_ref().map_or(false, |task| task.is_active()) {
                        html! {<div>{"Connecting..."}</div>}
                    } else if self.props.spotify_user.is_some() {
                        html! {
                            <>
                                <div>{"Your Spotify session has expired"}</div>
                                <br/>
                                <div><a href={url}>{"Re-connect with Spotify"}</a></div>
                            </>
                        }
                    } else {
                        html! {<div><a href={url}>{"Connect with Spotify"}</a></div>}
                    }
                }
            </div>
        }
    }
}

impl Connect {
    fn get_user_profile(&mut self, access_token: String, expiration_ts: i64) {
        let request = Request::get("https://api.spotify.com/v1/me")
            .header("Authorization", format!("Bearer {}", access_token))
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::fetch(
            request,
            self.link.callback(
                move |response: Response<Json<Result<SpotifyUserProfile, Error>>>| {
                    if let (meta, Json(Ok(user_profile))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::UserProfileLoaded(
                                user_profile.id,
                                access_token.clone(),
                                expiration_ts,
                            );
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
}

fn parse_query(query: &str) -> HashMap<String, Option<String>> {
    if query.len() > 0 {
        query[1..]
            .split('&')
            .map(|param| {
                let param_parts = param.split('=').collect::<Vec<_>>();
                if param_parts.len() == 2 {
                    (param_parts[0].to_string(), Some(param_parts[1].to_string()))
                } else if param_parts.len() == 1 {
                    (param_parts[0].to_string(), None)
                } else {
                    panic!("invalid location hash")
                }
            })
            .collect::<HashMap<String, Option<String>>>()
    } else {
        HashMap::new()
    }
}
