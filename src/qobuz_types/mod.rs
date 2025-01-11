use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, time::Duration};
use url::Url;
pub mod extra;
use extra::Extra;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist<T>
where
    Self: Extra,
{
    pub name: String,
    pub slug: String,
    pub owner: Owner,
    pub is_public: bool,

    #[serde(with = "ser_datetime_i64")]
    pub created_at: DateTime<Utc>, // TODO: Should NaiveDateTime be used instead?

    pub description: String,
    #[serde(with = "ser_duration_u64")]
    pub duration: Duration,
    pub genres: Vec<PlaylistGenre>,
    pub id: u64,
    pub images: Vec<Url>,
    pub images150: Vec<Url>,
    pub images300: Vec<Url>,
    pub is_collaborative: bool,
    pub is_featured: bool,
    pub updated_at: u64,
    pub users_count: u64,

    #[serde(flatten)]
    pub extra: T,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Owner {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Array<T> {
    pub items: Vec<T>,
    pub limit: i64,
    pub offset: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Track<T>
where
    Self: Extra,
{
    pub copyright: String,
    pub displayable: bool,
    pub downloadable: bool,
    #[serde(with = "ser_duration_u64")]
    pub duration: Duration,
    pub hires: bool,
    pub hires_streamable: bool,
    pub id: u64,
    pub isrc: String,
    // May be needed later. seems to represent CD number.
    // pub media_number: i64,
    pub parental_warning: bool,
    pub performer: Option<Performer>,
    pub performers: Option<String>,
    pub playlist_track_id: Option<i64>,
    pub position: Option<i64>,
    pub previewable: bool,
    pub purchasable: bool,
    pub release_date_original: NaiveDate,
    pub sampleable: bool,
    pub streamable: bool,
    pub title: String,
    pub track_number: u64,
    pub version: Option<String>,
    pub work: Option<String>,

    #[serde(flatten)]
    pub extra: T,
}

impl<T> Display for Track<T>
where
    Self: Extra,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (true, year) = self.release_date_original.year_ce() else {
            panic!("Release year shouldn't be BCE");
        };
        write!(
            f,
            "{} - {} ({})",
            self.performer
                .clone()
                .map_or("Various Artists".to_string(), |p| p.to_string()),
            self.title,
            year
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Album<T>
where
    Self: Extra,
{
    pub artist: Artist<()>,
    pub displayable: bool,
    pub downloadable: bool,
    #[serde(with = "ser_duration_u64")]
    pub duration: Duration,
    pub genre: Genre,
    pub hires: bool,
    pub hires_streamable: bool,
    pub image: Image,
    pub label: Label,
    // May be needed later. seems to represent number of CD's.
    // pub media_count: i64,
    pub id: String,
    pub release_date_original: NaiveDate,
    pub sampleable: bool,
    pub streamable: bool,
    pub title: String,
    pub upc: String,
    pub version: Option<String>,

    #[serde(flatten)]
    pub extra: T,
}

impl<T> Display for Album<T>
where
    Self: Extra,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} - {} ({})",
            self.artist,
            self.title,
            self.release_date_original.year()
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Artist<T>
where
    Self: Extra,
{
    pub albums_count: u64,
    pub id: i64,
    pub image: Value,
    pub name: String,
    pub slug: String,

    #[serde(flatten)]
    pub extra: T,
}

impl<T> Display for Artist<T>
where
    Self: Extra,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Genre {
    pub color: String,
    pub id: u64,
    pub name: String,
    // pub path: Vec<i64>,
    pub slug: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub large: String,
    pub small: String,
    pub thumbnail: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub albums_count: u64,
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub supplier_id: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Composer {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Performer {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlaylistGenre {
    String(String),
    Object {
        id: u32,
        color: String,
        name: String,
        path: Vec<u32>,
        slug: String,
        percent: f32,
    },
}

impl Display for Performer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub trait QobuzType: Serialize + for<'a> Deserialize<'a> + Extra {
    #[must_use]
    fn name_singular<'b>() -> &'b str;
    #[must_use]
    fn name_plural<'b>() -> &'b str;
}

impl<T> QobuzType for Album<T>
where
    Self: Extra,
{
    fn name_singular<'b>() -> &'b str {
        "album"
    }
    fn name_plural<'b>() -> &'b str {
        "albums"
    }
}

impl<T> QobuzType for Track<T>
where
    Self: Extra,
{
    fn name_singular<'b>() -> &'b str {
        "track"
    }
    fn name_plural<'b>() -> &'b str {
        "tracks"
    }
}

impl<T> QobuzType for Artist<T>
where
    Self: Extra,
{
    fn name_singular<'b>() -> &'b str {
        "artist"
    }
    fn name_plural<'b>() -> &'b str {
        "artists"
    }
}

impl<T> QobuzType for Playlist<T>
where
    Self: Extra,
{
    fn name_singular<'b>() -> &'b str {
        "playlist"
    }
    fn name_plural<'b>() -> &'b str {
        "playlists"
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
        Ok(DateTime::from_timestamp(i64::deserialize(deserializer)?, 0)
            .expect("Couldn't deserialize DateTime from timestamp"))
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
