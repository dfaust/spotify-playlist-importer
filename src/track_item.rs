use crate::playlist_types::*;
use yew::{html::Html, prelude::*, Properties};

use std::rc::Rc;

pub struct TrackItem {
    link: ComponentLink<Self>,
    props: Props,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub in_track: Rc<Track>,
    pub out_tracks: Rc<Vec<(f64, Track)>>,
    pub output_id: Option<String>,
    pub onmappingchange: Callback<(String, Option<String>)>,
    pub onquerytrack: Callback<(String, String)>,
}

pub enum Msg {
    OutTrackSelected(String),
    Noop,
}

impl Component for TrackItem {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        TrackItem { link, props }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::OutTrackSelected(output_id) => {
                if output_id == "search" {
                    let window = web_sys::window().expect("window not available");
                    let default_query = self.props.in_track.query();
                    if let Some(query) = window
                        .prompt_with_message_and_default(
                            "Please choose a search text:",
                            &default_query,
                        )
                        .expect("prompt not available")
                    {
                        self.props
                            .onquerytrack
                            .emit((self.props.in_track.id(), query));
                    }
                } else if output_id.is_empty() {
                    self.props
                        .onmappingchange
                        .emit((self.props.in_track.id(), None));
                } else {
                    self.props
                        .onmappingchange
                        .emit((self.props.in_track.id(), Some(output_id)));
                }
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
        let in_track = &self.props.in_track;
        let out_tracks = &self.props.out_tracks;
        let output_id = &self.props.output_id;

        let select_callback = self
            .link
            .callback(|event: yew::html::ChangeData| match event {
                yew::html::ChangeData::Select(select) => Msg::OutTrackSelected(select.value()),
                _ => Msg::Noop,
            });

        let render_select = html! {
            <select class="track" onchange=select_callback>
                {
                    if let Some(output_id) = output_id.as_ref() {
                        let track_found = out_tracks.iter().any(|(_, out_track)| out_track.id() == *output_id);
                        if track_found {
                            html! {<option value={""}>{"Don't import"}</option>}
                        } else {
                            html! {
                                <>
                                    <option value={""}>{"Don't import"}</option>
                                    <option value={output_id}>{"Loading data..."}</option>
                                </>
                            }
                        }
                    } else {
                        html! {<option value={""}>{"Don't import"}</option>}
                    }
                }
                {
                    for out_tracks.iter().map(|(similarity, out_track)| {
                        let value = out_track.id();
                        let text = format!(
                            "[{} %] {} - {} - {} ({})",
                            (similarity * 100.0).round(),
                            out_track.title.as_deref().unwrap_or_default(),
                            out_track.artist.as_deref().unwrap_or_default(),
                            out_track.album.as_deref().unwrap_or_default(),
                            format_duration(out_track.duration.unwrap_or_default()),
                        );
                        let selected = output_id.as_ref().map_or(false, |output_id| *output_id == out_track.id());
                        html! {
                            <option value={value} selected={selected}>{text}</option>
                        }
                    })
                }
                <option value={"search"}>{"Custom search..."}</option>
            </select>
        };

        html! {
            <tr>
                <td>{in_track.title.as_deref().unwrap_or_default()}</td>
                <td>{in_track.artist.as_deref().unwrap_or_default()}</td>
                <td>{in_track.album.as_deref().unwrap_or_default()}</td>
                <td class="right">{format_duration(in_track.duration.unwrap_or_default())}</td>
                <td>{render_select}</td>
            </tr>
        }
    }
}

fn format_duration(duration: i32) -> String {
    let hours = duration / 3_600_000;
    let minutes = (duration % 3_600_000) / 60_000;
    let seconds = (duration % 60_000) / 1_000;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}
