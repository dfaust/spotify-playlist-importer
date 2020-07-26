use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Playlist {
    pub title: Option<String>,
    pub annotation: Option<String>,
    #[serde(rename = "trackList")]
    pub track_list: TrackList,
}

impl Playlist {
    pub fn with_tracks_and_title(tracks: Vec<Track>, title: String) -> Playlist {
        Playlist {
            title: Some(title),
            track_list: TrackList { tracks },
            ..Default::default()
        }
    }

    pub fn to_xspf(&self) -> String {
        let tracks = self
            .track_list
            .tracks
            .iter()
            .map(Track::to_xspf)
            .collect::<Vec<_>>();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<playlist version="1" xmlns="http://xspf.org/ns/0/">
  <trackList>
{}
  </trackList>
</playlist>"#,
            tracks.join("\n")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct TrackList {
    #[serde(rename = "track")]
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Hash, Default)]
pub struct Track {
    pub location: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "creator")]
    pub artist: Option<String>,
    pub annotation: Option<String>,
    pub info: Option<String>,
    pub album: Option<String>,
    #[serde(rename = "trackNum")]
    pub track_number: Option<i32>,
    pub duration: Option<i32>,
}

impl Track {
    pub fn id(&self) -> String {
        self.identifier.clone().unwrap_or_else(|| {
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish().to_string()
        })
    }

    pub fn query(&self) -> String {
        let whitespaces = js_sys::RegExp::new(r"\s+", "");
        js_sys::JsString::from(
            format!(
                "{} {}",
                self.artist.as_deref().unwrap_or_default(),
                self.title.as_deref().unwrap_or_default()
            )
            .trim()
            .to_owned(),
        )
        .replace_by_pattern(&whitespaces, " ")
        .into()
    }

    pub fn adjusted_query(&self) -> String {
        let brackets = js_sys::RegExp::new(r"[\(\[].*[\)\]]", "");
        let artist = String::from(
            js_sys::JsString::from(self.artist.as_deref().unwrap_or_default())
                .replace_by_pattern(&brackets, ""),
        );
        let title = String::from(
            js_sys::JsString::from(self.title.as_deref().unwrap_or_default())
                .replace_by_pattern(&brackets, ""),
        );

        let whitespaces = js_sys::RegExp::new(r"\s+", "");
        js_sys::JsString::from(format!("{} {}", artist, title).trim().to_owned())
            .replace_by_pattern(&whitespaces, " ")
            .into()
    }

    pub fn similarity(&self, other: &Track) -> f64 {
        let artist_a = self.artist.as_deref().unwrap_or_default().to_lowercase();
        let artist_b = other.artist.as_deref().unwrap_or_default().to_lowercase();

        let album_a = self.album.as_deref().unwrap_or_default().to_lowercase();
        let album_b = other.album.as_deref().unwrap_or_default().to_lowercase();

        let title_a = self.title.as_deref().unwrap_or_default().to_lowercase();
        let title_b = other.title.as_deref().unwrap_or_default().to_lowercase();

        let duration_a = self.duration.unwrap_or_default();
        let duration_b = other.duration.unwrap_or_default();

        let duration_similarity = (1.0
            - f64::from(2 * (duration_a - duration_b).abs()) / f64::from(duration_a + duration_b))
        .powi(2);

        (strsim::jaro(&artist_a, &artist_b) * 2.0
            + strsim::jaro(&album_a, &album_b)
            + strsim::jaro(&title_a, &title_b) * 2.0
            + duration_similarity * 5.0)
            / 10.0
    }

    pub fn to_xspf(&self) -> String {
        let mut elements = Vec::with_capacity(9);
        if let Some(location) = self.location.as_ref() {
            elements.push(format!("      <location>{}</location>", location));
        }
        if let Some(identifier) = self.identifier.as_ref() {
            elements.push(format!("      <identifier>{}</identifier>", identifier));
        }
        if let Some(title) = self.title.as_ref() {
            elements.push(format!("      <title>{}</title>", title));
        }
        if let Some(artist) = self.artist.as_ref() {
            elements.push(format!("      <creator>{}</creator>", artist));
        }
        if let Some(annotation) = self.annotation.as_ref() {
            elements.push(format!("      <annotation>{}</annotation>", annotation));
        }
        if let Some(info) = self.info.as_ref() {
            elements.push(format!("      <info>{}</info>", info));
        }
        if let Some(album) = self.album.as_ref() {
            elements.push(format!("      <album>{}</album>", album));
        }
        if let Some(track_number) = self.track_number {
            elements.push(format!("      <trackNum>{}</trackNum>", track_number));
        }
        if let Some(duration) = self.duration {
            elements.push(format!("      <duration>{}</duration>", duration));
        }
        format!("    <track>\n{}\n    </track>", elements.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn query() {
        let track = Track {
            artist: Some("Artist".to_string()),
            title: Some("Title".to_string()),
            ..Default::default()
        };

        assert_eq!("Artist Title", track.query());
    }

    #[wasm_bindgen_test]
    fn adjusted_query() {
        let track = Track {
            artist: Some("Artist [Top]".to_string()),
            title: Some("Title (feat. Somebody)".to_string()),
            ..Default::default()
        };

        assert_eq!("Artist Title", track.adjusted_query());
    }
}
