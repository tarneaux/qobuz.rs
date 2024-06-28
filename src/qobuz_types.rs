use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, time::Duration};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    #[serde(rename = "created_at", with = "ser_datetime_i64")]
    pub created_at: DateTime<Utc>, // TODO: Should NaiveDateTime be used instead?
    pub description: String,
    #[serde(with = "ser_duration_u64")]
    pub duration: Duration,
    pub genres: Vec<String>,
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
    pub slug: String,
    pub tracks: Option<Array<Track>>,
    #[serde(rename = "updated_at")]
    pub updated_at: u64,
    #[serde(rename = "users_count")]
    pub users_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Owner {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Array<T> {
    pub items: Vec<T>,
    pub limit: i64,
    pub offset: i64,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub album: Option<Album>,
    pub composer: Option<Composer>,
    pub copyright: String,
    pub displayable: bool,
    pub downloadable: bool,
    #[serde(with = "ser_duration_u64")]
    pub duration: Duration,
    pub hires: bool,
    #[serde(rename = "hires_streamable")]
    pub hires_streamable: bool,
    pub id: u64,
    pub isrc: String,
    // May be needed later. seems to represent CD number.
    // #[serde(rename = "media_number")]
    // pub media_number: i64,
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
    pub title: String,
    #[serde(rename = "track_number")]
    pub track_number: u64,
    pub version: Option<String>,
    pub work: Option<String>,
}

impl Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub artist: Artist,
    pub displayable: bool,
    pub downloadable: bool,
    #[serde(with = "ser_duration_u64")]
    pub duration: Duration,
    pub genre: Genre,
    pub hires: bool,
    #[serde(rename = "hires_streamable")]
    pub hires_streamable: bool,
    pub image: Image,
    pub label: Label,
    // May be needed later. seems to represent number of CD's.
    // #[serde(rename = "media_count")]
    // pub media_count: i64,
    pub id: String,
    #[serde(rename = "release_date_original")]
    pub released: NaiveDate,
    pub sampleable: bool,
    pub streamable: bool,
    pub title: String,
    pub upc: String,
    pub version: Option<String>,
    pub tracks: Option<Array<Track>>,
}

impl Display for Album {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} - {} ({})",
            self.artist,
            self.title,
            self.released.year()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    #[serde(rename = "albums_count")]
    pub albums_count: u64,
    pub id: i64,
    pub image: Value,
    pub name: String,
    pub slug: String,
    pub albums: Option<Array<Album>>,
}

impl Display for Artist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub color: String,
    pub id: u64,
    pub name: String,
    // pub path: Vec<i64>,
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
    pub albums_count: u64,
    pub id: u64,
    pub name: String,
    pub slug: String,
    #[serde(rename = "supplier_id")]
    pub supplier_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Composer {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Performer {
    pub id: u64,
    pub name: String,
}

impl Display for Performer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub trait QobuzType: Serialize + for<'a> Deserialize<'a> {
    fn name_plural<'b>() -> &'b str;
}

impl QobuzType for Album {
    fn name_plural<'b>() -> &'b str {
        "albums"
    }
}

impl QobuzType for Track {
    fn name_plural<'b>() -> &'b str {
        "tracks"
    }
}

impl QobuzType for Artist {
    fn name_plural<'b>() -> &'b str {
        "artists"
    }
}

mod ser_datetime_i64 {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(datetime: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        datetime.timestamp().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(DateTime::from_timestamp(i64::deserialize(deserializer)?, 0).unwrap())
    }
}

mod ser_duration_u64 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Duration::from_secs(u64::deserialize(deserializer)?))
    }
}
