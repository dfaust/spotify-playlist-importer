use crate::playlist_types::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyArtist {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyAlbum {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyTrack {
    pub uri: String,
    pub album: SpotifyAlbum,
    pub artists: Vec<SpotifyArtist>,
    pub name: String,
    pub track_number: i32,
    pub duration_ms: i32,
}

impl From<SpotifyTrack> for Track {
    fn from(f: SpotifyTrack) -> Track {
        Track {
            identifier: Some(f.uri),
            title: Some(f.name),
            track_number: Some(f.track_number),
            duration: Some(f.duration_ms),
            artist: Some(
                f.artists
                    .into_iter()
                    .map(|artist| artist.name)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            album: Some(f.album.name),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyTracks {
    pub tracks: Vec<SpotifyTrack>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyUser {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub owner: SpotifyUser,
    pub collaborative: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyUserProfile {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyPagination<T> {
    pub items: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyResult {
    pub tracks: SpotifyPagination<SpotifyTrack>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyCreatePlaylist {
    pub name: String,
    pub public: bool,
}
