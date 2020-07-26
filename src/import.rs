use crate::app::SpotifyUser;
use crate::download_file;
use crate::playlist_types::*;
use crate::spotify_types::{
    SpotifyCreatePlaylist, SpotifyPagination, SpotifyPlaylist, SpotifyResult, SpotifyTracks,
};
use crate::TrackList;

use anyhow::Error;
use http::{Request, Response};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use yew::services::fetch::{FetchService, FetchTask};
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::services::storage::{Area, StorageService};
use yew::services::{interval::IntervalTask, IntervalService, Task};
use yew::{
    format::{Json, Nothing},
    html::Html,
    prelude::*,
    Properties,
};

use std::collections::{HashMap, VecDeque};
use std::{rc::Rc, time::Duration};

const LS_ID_MAPPING: &str = "id-mapping";

pub struct Import {
    link: ComponentLink<Self>,
    storage: StorageService,
    reader: ReaderService,
    props: Props,
    state: State,
    fetch_tasks: Vec<FetchTask>,
    reader_tasks: Vec<ReaderTask>,
    _interval_task: IntervalTask,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub spotify_user: Rc<SpotifyUser>,
}

#[derive(Clone, Copy)]
pub enum FetchInitiator {
    Auto(usize),
    Manual,
}

pub struct State {
    in_tracks: Rc<Vec<Rc<Track>>>,
    out_tracks: Rc<HashMap<String, Rc<Vec<(f64, Track)>>>>,
    id_mapping: Rc<HashMap<String, String>>,
    out_playlists: Vec<SpotifyPlaylist>,
    selected_out_playlist: Option<String>,
    fetch_out_tracks_queue: VecDeque<(String, String, FetchInitiator)>,
    fetch_out_tracks_remaining: HashMap<String, String>,
    fetch_out_tracks_remaining_batch_index: usize,
    import_matched_batch_index: usize,
    import_matched_done: bool,
    error_message: Option<String>,
}

pub enum Msg {
    OutPlaylistsLoaded(Vec<SpotifyPlaylist>),
    OutPlaylistSelected(String),
    OutPlaylistCreated(SpotifyPlaylist),
    InPlaylistSelected(File),
    InPlaylistLoaded(FileData),
    SetIdMapping(String, Option<String>),
    OutTracksFound(String, Vec<Track>, FetchInitiator),
    RemainingOutTracksFound(Vec<(String, Track)>),
    QueryOutTrack(String, String),
    ExportUnmatched,
    ImportMatched,
    ImportMatchedDone,
    SetError(String),
    Noop,
}

impl Component for Import {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let id_mapping = {
            if let Json(Ok(restored_id_mapping)) = storage.restore(LS_ID_MAPPING) {
                Rc::new(restored_id_mapping)
            } else {
                Rc::new(HashMap::new())
            }
        };
        let state = State {
            in_tracks: Rc::new(Vec::new()),
            out_tracks: Rc::new(HashMap::new()),
            id_mapping,
            out_playlists: Vec::new(),
            selected_out_playlist: None,
            fetch_out_tracks_queue: VecDeque::new(),
            fetch_out_tracks_remaining: HashMap::new(),
            fetch_out_tracks_remaining_batch_index: 0,
            import_matched_batch_index: 0,
            import_matched_done: false,
            error_message: None,
        };
        let _interval_task = IntervalService::spawn(Duration::from_secs(60), link.callback(|_| Msg::Noop));
        let mut import = Import {
            link,
            storage,
            reader: ReaderService::new(),
            props,
            state,
            fetch_tasks: Vec::new(),
            reader_tasks: Vec::new(),
            _interval_task,
        };
        import.get_playlists();
        import
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::OutPlaylistsLoaded(playlists) => {
                self.state.error_message = None;
                self.state.out_playlists = playlists;
            }
            Msg::OutPlaylistSelected(playlist_id) => {
                if playlist_id == "create" {
                    let window = web_sys::window().expect("window not available");
                    if let Some(name) = window
                        .prompt_with_message("Please enter a name for the playlist:")
                        .expect("prompt not available")
                    {
                        self.create_playlist(name);
                    }
                } else if !playlist_id.is_empty() {
                    self.state.selected_out_playlist = Some(playlist_id);
                } else {
                    self.state.selected_out_playlist = None;
                }
            }
            Msg::OutPlaylistCreated(playlist) => {
                self.state.selected_out_playlist = Some(playlist.id.clone());
                self.state.out_playlists.push(playlist);
            }
            Msg::InPlaylistSelected(file) => {
                let callback = self.link.callback(Msg::InPlaylistLoaded);
                let reader_task = self.reader.read_file(file, callback).unwrap();
                self.reader_tasks.push(reader_task);
            }
            Msg::InPlaylistLoaded(file_data) => {
                let playlist: Playlist = serde_xml_rs::from_reader(&file_data.content[..])
                    .expect("deserialize playlist"); // TODO error handling
                self.state.in_tracks = Rc::new(
                    playlist
                        .track_list
                        .tracks
                        .into_iter()
                        .map(Rc::new)
                        .collect(),
                );

                for in_track in self.state.in_tracks.iter() {
                    self.state.fetch_out_tracks_queue.push_back((
                        in_track.id(),
                        in_track.query(),
                        FetchInitiator::Auto(1),
                    ));
                }

                self.state.fetch_out_tracks_remaining.clear();
                self.state.fetch_out_tracks_remaining_batch_index = 0;

                for in_track in self.state.in_tracks.iter() {
                    let input_id = in_track.id();
                    if let Some(output_id) = self.state.id_mapping.get(&input_id) {
                        if !self.state.out_tracks.contains_key(output_id) {
                            self.state
                                .fetch_out_tracks_remaining
                                .insert(input_id, output_id.clone());
                        }
                    }
                }

                self.fetch_next_out_track();
            }
            Msg::SetIdMapping(input_id, Some(output_id)) => {
                Rc::make_mut(&mut self.state.id_mapping).insert(input_id, output_id);
                self.storage
                    .store(LS_ID_MAPPING, Json(&*self.state.id_mapping));
            }
            Msg::SetIdMapping(input_id, None) => {
                Rc::make_mut(&mut self.state.id_mapping).remove(&input_id);
                self.storage
                    .store(LS_ID_MAPPING, Json(&*self.state.id_mapping));
            }
            Msg::QueryOutTrack(input_id, query) => {
                self.state.fetch_out_tracks_queue.push_back((
                    input_id,
                    query,
                    FetchInitiator::Manual,
                ));
                self.fetch_next_out_track();
            }
            Msg::OutTracksFound(input_id, new_out_tracks, fetch_initiator) => {
                self.state.error_message = None;

                if new_out_tracks.len() > 0 {
                    self.insert_out_track(input_id, new_out_tracks);
                } else {
                    if let FetchInitiator::Auto(fetch_try) = fetch_initiator {
                        if fetch_try == 1 {
                            let in_track = self
                                .state
                                .in_tracks
                                .iter()
                                .find(|in_track| in_track.id() == *input_id)
                                .expect("received track for invalid input");

                            let query = in_track.query();
                            let adjusted_query = in_track.adjusted_query();

                            if adjusted_query != query {
                                self.state.fetch_out_tracks_queue.push_back((
                                    in_track.id(),
                                    adjusted_query,
                                    FetchInitiator::Auto(fetch_try + 1),
                                ));
                            }
                        }
                    }
                }

                self.fetch_next_out_track();
            }
            Msg::RemainingOutTracksFound(tracks) => {
                self.state.error_message = None;

                for (input_id, new_out_track) in tracks {
                    self.insert_out_track(input_id, vec![new_out_track]);
                }

                self.fetch_next_out_track();
            }
            Msg::ExportUnmatched => {
                let tracks = self
                    .state
                    .in_tracks
                    .iter()
                    .filter(|in_track| !self.state.id_mapping.contains_key(&in_track.id()))
                    .map(|in_track| in_track.as_ref().clone())
                    .collect::<Vec<_>>();
                let playlist = Playlist::with_tracks_and_title(
                    tracks,
                    "spotify-playlist-importer".to_string(),
                );
                let xspf = playlist.to_xspf();
                unsafe {
                    download_file("spotify-playlist-importer.xspf", &xspf);
                }
            }
            Msg::ImportMatched => {
                if let Some(playlist_id) = self.state.selected_out_playlist.clone() {
                    self.state.import_matched_batch_index = 0;
                    self.state.import_matched_done = false;
                    self.add_next_to_playlist(&playlist_id);
                }
            }
            Msg::ImportMatchedDone => {
                self.state.error_message = None;
                self.state.import_matched_done = true;
            }
            Msg::SetError(error_message) => {
                self.state.error_message = Some(error_message);
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
        let onchange_in_playlist = self
            .link
            .callback(|event: yew::html::ChangeData| match event {
                yew::html::ChangeData::Files(files) => match files.get(0) {
                    Some(file) => Msg::InPlaylistSelected(file),
                    None => Msg::Noop,
                },
                _ => Msg::Noop,
            });
        let onchange_out_playlist =
            self.link
                .callback(|event: yew::html::ChangeData| match event {
                    yew::html::ChangeData::Select(select) => {
                        Msg::OutPlaylistSelected(select.value())
                    }
                    _ => Msg::Noop,
                });
        let onclick_import_matched = self.link.callback(|_| Msg::ImportMatched);
        let onclick_export_unmatched = self.link.callback(|_| Msg::ExportUnmatched);

        let onmappingchange = self
            .link
            .callback(|(input_id, output_id)| Msg::SetIdMapping(input_id, output_id));
        let onquerytrack = self
            .link
            .callback(|(input_id, query)| Msg::QueryOutTrack(input_id, query));

        let render_error_message = if let Some(error_message) = self.state.error_message.as_ref() {
            html! {<div class="error">{error_message}</div>}
        } else {
            html! {}
        };

        let render_is_loading = if self.fetch_tasks.iter().any(FetchTask::is_active) {
            html! {<div class="inline lds-dual-ring"/>}
        } else {
            html! {}
        };

        let render_is_submitting = if self.fetch_tasks.iter().any(FetchTask::is_active) {
            html! {<div class="inline lds-dual-ring"/>}
        } else {
            html! {}
        };

        let render_message = if self.state.import_matched_done {
            html! {<div class="inline success">{"✔ Import succeeded"}</div>}
        } else {
            html! {}
        };

        html! {
            <div>
                {render_error_message}
                <div>
                    <span class="form">{"Input playlist:"}</span>
                    <input class="inline" type="file" onchange=onchange_in_playlist/>
                    {render_is_loading}
                </div>
                <br/>
                <TrackList
                    in_tracks=self.state.in_tracks.clone()
                    out_tracks=self.state.out_tracks.clone()
                    id_mapping=self.state.id_mapping.clone()
                    onmappingchange=onmappingchange
                    onquerytrack=onquerytrack
                />
                <br/>
                <div>
                    <span class="form">{"Spotify playlist:"}</span>
                    <select onchange=onchange_out_playlist class="inline playlist">
                        {
                            if self.state.selected_out_playlist.is_none() {
                                html! {<option value={""}>{"Select playlist"}</option>}
                            } else {
                                html! {}
                            }
                        }
                        {
                            for self.state.out_playlists.iter().map(|playlist| {
                                let value = &playlist.id;
                                let text = &playlist.name;
                                let selected = self.state.selected_out_playlist.as_ref().map_or(false, |selected_out_playlist| selected_out_playlist == &playlist.id);
                                html! {
                                    <option value={value} selected={selected}>{text}</option>
                                }
                            })
                        }
                        <option value={"create"}>{"Create new playlist..."}</option>
                    </select>
                </div>
                <br/>
                <div>
                    <button
                        class="main"
                        onclick=onclick_import_matched
                        disabled=self.state.in_tracks.is_empty() || self.state.selected_out_playlist.is_none()
                    >
                        {"Import playlist"}
                    </button>
                    <button
                        onclick=onclick_export_unmatched
                        disabled=self.state.in_tracks.is_empty()
                    >
                        {"Export unmatched"}
                    </button>
                    {render_message}
                    {render_is_submitting}
                </div>
                <br/>
                <div class="status">
                    {format!("Your Spotify session will expire in {} minutes", self.props.spotify_user.expiration_timeout() / 1_000 / 60)}
                </div>
            </div>
        }
    }
}

impl Import {
    fn insert_out_track(&mut self, input_id: String, new_out_tracks: Vec<Track>) {
        let in_track = self
            .state
            .in_tracks
            .iter()
            .find(|in_track| in_track.id() == *input_id)
            .expect("received track for invalid input");

        // append out tracks
        let mut new_out_tracks = new_out_tracks
            .into_iter()
            .map(|out_track| (in_track.similarity(&out_track), out_track))
            .collect::<Vec<_>>();
        let out_tracks = Rc::make_mut(
            Rc::make_mut(&mut self.state.out_tracks)
                .entry(input_id.clone())
                .or_default(),
        );
        out_tracks.append(&mut new_out_tracks);
        out_tracks.sort_by_key(|(similarity, _)| -(similarity * 1_000.0) as isize);

        // set default mapping
        if !self.state.id_mapping.contains_key(&input_id) {
            Rc::make_mut(&mut self.state.id_mapping).insert(input_id.clone(), out_tracks[0].1.id());
            self.storage
                .store(LS_ID_MAPPING, Json(&*self.state.id_mapping));
        }

        // remove from remaining
        if let Some(output_id) = self.state.fetch_out_tracks_remaining.get(&input_id) {
            if out_tracks
                .iter()
                .any(|(_, out_track)| out_track.id() == *output_id)
            {
                self.state.fetch_out_tracks_remaining.remove(&input_id);
            }
        }
    }

    fn get_playlists(&mut self) {
        let request = Request::get(format!(
            "https://api.spotify.com/v1/users/{}/playlists?limit=50",
            self.props.spotify_user.user_id
        ))
        .header(
            "Authorization",
            format!("Bearer {}", self.props.spotify_user.access_token),
        )
        .body(Nothing)
        .expect("failed to build request");

        let spotify_user_id = self.props.spotify_user.user_id.clone();

        if let Ok(task) = FetchService::fetch(
            request,
            self.link.callback(
                move |response: Response<
                    Json<Result<SpotifyPagination<SpotifyPlaylist>, Error>>,
                >| {
                    if let (meta, Json(Ok(playlists))) = response.into_parts() {
                        if meta.status.is_success() {
                            let playlists = playlists
                                .items
                                .into_iter()
                                .filter(|playlist| {
                                    playlist.owner.id == spotify_user_id || playlist.collaborative
                                })
                                .collect();
                            return Msg::OutPlaylistsLoaded(playlists);
                        }
                    }
                    Msg::SetError("Request failed: get playlists".to_string())
                },
            ),
        ) {
            self.fetch_tasks.push(FetchTask::from(task));
        }
    }

    fn fetch_next_out_track(&mut self) {
        self.fetch_tasks.retain(FetchTask::is_active);

        while self.fetch_tasks.len() < 1 {
            match self.state.fetch_out_tracks_queue.pop_front() {
                Some((input_id, query, fetch_initiator)) => {
                    self.fetch_out_track(input_id, query, fetch_initiator);
                }
                None => break,
            }
        }

        let batch_count =
            (self.state.fetch_out_tracks_remaining.len() as f64 / 50.0).ceil() as usize;

        if self.fetch_tasks.len() == 0
            && self.state.fetch_out_tracks_remaining_batch_index < batch_count
        {
            self.fetch_remaining_out_tracks();
            self.state.fetch_out_tracks_remaining_batch_index += 1;
        }
    }

    fn fetch_out_track(
        &mut self,
        input_id: String,
        query: String,
        fetch_initiator: FetchInitiator,
    ) {
        let request = Request::get(format!(
            "https://api.spotify.com/v1/search?q={}&type=track",
            utf8_percent_encode(&query, NON_ALPHANUMERIC)
        ))
        .header(
            "Authorization",
            format!("Bearer {}", self.props.spotify_user.access_token),
        )
        .body(Nothing)
        .expect("failed to build request");

        if let Ok(task) = FetchService::fetch(
            request,
            self.link.callback(
                move |response: Response<Json<Result<SpotifyResult, Error>>>| {
                    if let (meta, Json(Ok(result))) = response.into_parts() {
                        if meta.status.is_success() {
                            let tracks = result
                                .tracks
                                .items
                                .into_iter()
                                .map(Into::into)
                                .collect::<Vec<Track>>();
                            return Msg::OutTracksFound(input_id.clone(), tracks, fetch_initiator);
                        }
                    }
                    Msg::SetError("Request failed: search track".to_string())
                },
            ),
        ) {
            self.fetch_tasks.push(FetchTask::from(task));
        }
    }

    fn fetch_remaining_out_tracks(&mut self) {
        let spotify_ids = self
            .state
            .fetch_out_tracks_remaining
            .values()
            .skip(self.state.fetch_out_tracks_remaining_batch_index * 50)
            .take(50)
            .map(AsRef::as_ref)
            .map(parse_spotify_id)
            .collect::<Vec<_>>()
            .join(",");

        let request = Request::get(format!(
            "https://api.spotify.com/v1/tracks/?ids={}",
            spotify_ids
        ))
        .header(
            "Authorization",
            format!("Bearer {}", self.props.spotify_user.access_token),
        )
        .body(Nothing)
        .expect("failed to build request");

        let id_lookup = self
            .state
            .fetch_out_tracks_remaining
            .iter()
            .skip(self.state.fetch_out_tracks_remaining_batch_index * 50)
            .take(50)
            .map(|(input_id, output_id)| (output_id.clone(), input_id.clone()))
            .collect::<HashMap<_, _>>();

        if let Ok(task) = FetchService::fetch(
            request,
            self.link.callback(
                move |response: Response<Json<Result<SpotifyTracks, Error>>>| {
                    if let (meta, Json(Ok(tracks))) = response.into_parts() {
                        if meta.status.is_success() {
                            let tracks = tracks
                                .tracks
                                .into_iter()
                                .map(|track| {
                                    let out_track = Track::from(track);
                                    let output_id = out_track.id();
                                    let input_id = id_lookup
                                        .get(&output_id)
                                        .expect("received unexpected track")
                                        .to_string();
                                    (input_id, out_track)
                                })
                                .collect::<Vec<_>>();
                            return Msg::RemainingOutTracksFound(tracks);
                        }
                    }
                    Msg::SetError("Request failed: get tracks".to_string())
                },
            ),
        ) {
            self.fetch_tasks.push(FetchTask::from(task));
        }
    }

    fn add_next_to_playlist(&mut self, playlist_id: &str) {
        let batch_count = (self.state.in_tracks.len() as f64 / 50.0).ceil() as usize;

        if self.state.import_matched_batch_index < batch_count {
            self.add_to_playlist(playlist_id);
            self.state.import_matched_batch_index += 1;
        }
    }

    fn add_to_playlist(&mut self, playlist_id: &str) {
        let uris = self
            .state
            .in_tracks
            .iter()
            .skip(self.state.import_matched_batch_index * 50)
            .take(50)
            .filter_map(|in_track| self.state.id_mapping.get(&in_track.id()))
            .cloned()
            .collect::<Vec<_>>();
        let body_json = serde_json::json!({ "uris": uris });

        let request = Request::post(format!(
            "https://api.spotify.com/v1/playlists/{}/tracks",
            playlist_id
        ))
        .header(
            "Authorization",
            format!("Bearer {}", self.props.spotify_user.access_token),
        )
        .body(Json(&body_json))
        .expect("failed to build request");

        if let Ok(task) = FetchService::fetch(
            request,
            self.link
                .callback(move |response: Response<Result<String, Error>>| {
                    if response.status().is_success() {
                        return Msg::ImportMatchedDone;
                    }
                    Msg::SetError("Request failed: add to playlist".to_string())
                }),
        ) {
            self.fetch_tasks.push(FetchTask::from(task));
        }
    }

    fn create_playlist(&mut self, name: String) {
        let playlist = SpotifyCreatePlaylist {
            name,
            public: false,
        };
        let body_json = serde_json::json!(playlist);

        let request = Request::post(format!(
            "https://api.spotify.com/v1/users/{}/playlists",
            self.props.spotify_user.user_id
        ))
        .header(
            "Authorization",
            format!("Bearer {}", self.props.spotify_user.access_token),
        )
        .body(Json(&body_json))
        .expect("failed to build request");

        if let Ok(task) = FetchService::fetch(
            request,
            self.link.callback(
                move |response: Response<Json<Result<SpotifyPlaylist, Error>>>| {
                    if let (meta, Json(Ok(playlist))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::OutPlaylistCreated(playlist);
                        }
                    }
                    Msg::SetError("Request failed: create playlist".to_string())
                },
            ),
        ) {
            self.fetch_tasks.push(FetchTask::from(task));
        }
    }
}

fn parse_spotify_id(uri: &str) -> &str {
    uri.split(':').last().expect("invalid spotify uri")
}
