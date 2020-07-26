use crate::playlist_types::*;
use crate::TrackItem;
use yew::{html::Html, prelude::*, Properties};

use std::collections::HashMap;
use std::rc::Rc;

pub struct TrackList {
    props: Props,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub in_tracks: Rc<Vec<Rc<Track>>>,
    pub out_tracks: Rc<HashMap<String, Rc<Vec<(f64, Track)>>>>,
    pub id_mapping: Rc<HashMap<String, String>>,
    pub onmappingchange: Callback<(String, Option<String>)>,
    pub onquerytrack: Callback<(String, String)>,
}

pub enum Msg {
    Noop,
}

impl Component for TrackList {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        TrackList { props }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Noop => {}
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <table>
                    <thead>
                        <tr>
                            <th>{"Title"}</th>
                            <th>{"Artist"}</th>
                            <th>{"Album"}</th>
                            <th class="right">{"Length"}</th>
                            <th>{"Spotify"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {
                            for self.props.in_tracks.iter().cloned().map(|in_track| {
                                let input_id = in_track.id();
                                let out_tracks = self.props.out_tracks.get(&input_id).cloned().unwrap_or_default();
                                let output_id = self.props.id_mapping.get(&input_id).cloned();
                                html! {
                                    <TrackItem
                                        in_track=in_track
                                        out_tracks=out_tracks
                                        output_id=output_id
                                        onmappingchange=self.props.onmappingchange.clone()
                                        onquerytrack=self.props.onquerytrack.clone()
                                    />}
                            })
                        }
                    </tbody>
                </table>
            </div>
        }
    }
}
