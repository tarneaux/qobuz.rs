use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    #[serde(rename = "created_at")]
    pub created_at: u64,
    pub description: String,
    pub duration: u64,
    pub genres: Vec<Value>,
    pub id: u64,
    pub images: Vec<String>,
    pub images150: Vec<String>,
    pub images300: Vec<String>,
    #[serde(rename = "is_collaborative")]
    pub is_collaborative: bool,
    #[serde(rename = "is_featured")]
    pub is_featured: bool,
    #[serde(rename = "is_public")]
    pub is_public: bool,
    pub name: String,
    pub owner: Owner,
    #[serde(rename = "published_from")]
    pub published_from: Value,
    #[serde(rename = "published_to")]
    pub published_to: Value,
    pub slug: String,
    pub tracks: Tracks,
    #[serde(rename = "tracks_count")]
    pub tracks_count: u64,
    #[serde(rename = "updated_at")]
    pub updated_at: u64,
    #[serde(rename = "users_count")]
    pub users_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    pub items: Vec<Track>,
    pub limit: i64,
    pub offset: i64,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub album: Album,
    #[serde(rename = "audio_info")]
    pub audio_info: AudioInfo,
    pub composer: Option<Composer>,
    pub copyright: String,
    // #[serde(rename = "created_at")]
    // pub created_at: i64,
    pub displayable: bool,
    pub downloadable: bool,
    pub duration: i64,
    pub hires: bool,
    #[serde(rename = "hires_streamable")]
    pub hires_streamable: bool,
    pub id: i64,
    pub isrc: String,
    #[serde(rename = "maximum_bit_depth")]
    pub maximum_bit_depth: i64,
    #[serde(rename = "maximum_channel_count")]
    pub maximum_channel_count: i64,
    #[serde(rename = "maximum_sampling_rate")]
    pub maximum_sampling_rate: f64,
    #[serde(rename = "media_number")]
    pub media_number: i64,
    #[serde(rename = "parental_warning")]
    pub parental_warning: bool,
    pub performer: Performer,
    pub performers: String,
    #[serde(rename = "playlist_track_id")]
    pub playlist_track_id: Option<i64>,
    pub position: Option<i64>,
    pub previewable: bool,
    pub purchasable: bool,
    #[serde(rename = "release_date_original")]
    pub released: NaiveDate,
    pub sampleable: bool,
    pub streamable: bool,
    #[serde(rename = "streamable_at")]
    pub streamable_at: i64,
    pub title: String,
    #[serde(rename = "track_number")]
    pub track_number: i64,
    pub version: Option<String>,
    pub work: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub artist: Artist,
    pub displayable: bool,
    pub downloadable: bool,
    pub duration: i64,
    pub genre: Genre,
    pub hires: bool,
    #[serde(rename = "hires_streamable")]
    pub hires_streamable: bool,
    pub image: Image,
    pub label: Label,
    #[serde(rename = "media_count")]
    pub media_count: i64,
    #[serde(rename = "qobuz_id")]
    pub id: i64,
    #[serde(rename = "release_date_original")]
    pub released: NaiveDate,
    pub sampleable: bool,
    pub streamable: bool,
    #[serde(rename = "streamable_at")]
    pub streamable_at: i64,
    pub title: String,
    #[serde(rename = "tracks_count")]
    pub tracks_count: i64,
    pub upc: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    #[serde(rename = "albums_count")]
    pub albums_count: i64,
    pub id: i64,
    pub image: Value,
    pub name: String,
    pub picture: Value,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub color: String,
    pub id: i64,
    pub name: String,
    pub path: Vec<i64>,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub large: String,
    pub small: String,
    pub thumbnail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    #[serde(rename = "albums_count")]
    pub albums_count: i64,
    pub id: i64,
    pub name: String,
    pub slug: String,
    #[serde(rename = "supplier_id")]
    pub supplier_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInfo {
    #[serde(rename = "replaygain_track_gain")]
    pub replaygain_track_gain: f64,
    #[serde(rename = "replaygain_track_peak")]
    pub replaygain_track_peak: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Composer {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Performer {
    pub id: i64,
    pub name: String,
}
