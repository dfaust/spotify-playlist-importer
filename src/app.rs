use yew::format::Json;
use yew::prelude::*;
use yew::services::storage::{Area, StorageService};

use std::rc::Rc;

use crate::{Connect, Import};

const LS_SPOTIFY_USER: &str = "spotify-user";

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct SpotifyUser {
    pub user_id: String,
    pub access_token: String,
    pub expiration_ts: i64,
}

impl SpotifyUser {
    pub fn expiration_timeout(&self) -> i64 {
        let now = js_sys::Date::now() as i64;
        self.expiration_ts - now
    }
}

pub struct App {
    link: ComponentLink<Self>,
    storage: StorageService,
    state: State,
}

pub struct State {
    spotify_user: Option<Rc<SpotifyUser>>,
}

pub enum Msg {
    SetSpotifyUser(SpotifyUser),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let spotify_user = {
            if let Json(Ok(restored_spotify_user)) = storage.restore(LS_SPOTIFY_USER) {
                Some(Rc::new(restored_spotify_user))
            } else {
                None
            }
        };
        let state = State { spotify_user };
        App {
            link,
            storage,
            state,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetSpotifyUser(spotify_user) => {
                self.state.spotify_user = Some(Rc::new(spotify_user));
                self.storage
                    .store(LS_SPOTIFY_USER, Json(&self.state.spotify_user.as_deref()));
            }
        }
        true
    }

    fn view(&self) -> Html {
        if let Some(spotify_user) = self
            .state
            .spotify_user
            .as_ref()
            .filter(|u| u.expiration_timeout() > 0)
            .to_owned()
        {
            html! {
                <main>
                    <Import spotify_user=spotify_user />
                </main>
            }
        } else {
            let onconnect = self
                .link
                .callback(|spotify_user| Msg::SetSpotifyUser(spotify_user));
            html! {
                <main>
                    <Connect spotify_user=self.state.spotify_user.clone() onconnect=onconnect />
                </main>
            }
        }
    }
}
