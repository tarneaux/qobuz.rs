// Erroneous warning that is shown when using the same trait twice with different arguments
#![allow(clippy::trait_duplication_in_bounds)]

pub mod extra;
pub mod traits;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use extra::{ExtraFlag, WithExtra, WithoutExtra};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, time::Duration};
use url::Url;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist<EF: ExtraFlag<Array<Track<WithExtra>>>> {
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
    pub tracks: EF::Extra,
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
pub struct Track<EF>
where
    EF: ExtraFlag<Album<WithoutExtra>>,
{
    pub copyright: Option<String>,
    pub displayable: bool,
    pub downloadable: bool,
    #[serde(with = "ser_duration_u64")]
    pub duration: Duration,
    pub hires: bool,
    pub hires_streamable: bool,
    pub id: u64,
    pub isrc: String,
    pub media_number: u64,
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
    pub album: EF::Extra,
}

impl<EF> Display for Track<EF>
where
    EF: ExtraFlag<Album<WithoutExtra>>,
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
pub struct Album<EF>
where
    EF: ExtraFlag<Array<Track<WithoutExtra>>>,
{
    pub artist: Artist<WithoutExtra>,
    pub displayable: bool,
    pub downloadable: bool,
    #[serde(with = "ser_duration_u64")]
    pub duration: Duration,
    pub genre: Genre,
    pub hires: bool,
    pub hires_streamable: bool,
    pub image: Image,
    pub label: Label,
    pub media_count: i64,
    pub id: String,
    pub release_date_original: NaiveDate,
    pub sampleable: bool,
    pub streamable: bool,
    pub title: String,
    pub upc: String,
    pub version: Option<String>,
    pub tracks: EF::Extra,
}

impl<EF> Display for Album<EF>
where
    EF: ExtraFlag<Array<Track<WithoutExtra>>>,
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

impl Album<WithExtra> {
    #[must_use]
    pub fn get_tracks_with_extra(&self) -> Vec<Track<WithExtra>> {
        let s = self.clone().without_extra();
        self.tracks
            .items
            .iter()
            .map(|t| -> Track<WithExtra> { t.clone().with_extra(s.clone()) })
            .collect()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Artist<EF>
where
    EF: ExtraFlag<Array<Track<WithExtra>>> + ExtraFlag<Array<Album<WithoutExtra>>>,
{
    pub albums_count: u64,
    pub id: i64,
    pub image: Value,
    pub name: String,
    pub slug: String,
    pub tracks: <EF as ExtraFlag<Array<Track<WithExtra>>>>::Extra,
    pub albums: <EF as ExtraFlag<Array<Album<WithoutExtra>>>>::Extra,
}

impl<EF> Display for Artist<EF>
where
    EF: ExtraFlag<Array<Track<WithExtra>>> + ExtraFlag<Array<Album<WithoutExtra>>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Genre {
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

pub trait QobuzType: std::fmt::Debug {
    #[must_use]
    fn name_singular<'b>() -> &'b str
    where
        Self: Sized;
    #[must_use]
    fn name_plural<'b>() -> &'b str
    where
        Self: Sized;
}

macro_rules! impl_qobuz_type {
    (
        $t:ident,
        [$( $extra_type:ty ),+]
    ) => {
        paste::paste! {
            impl<EF> QobuzType for $t<EF>
            where
                $( EF: ExtraFlag<$extra_type>, )+
                Self: std::fmt::Debug,
            {
                fn name_singular<'b>() -> &'b str {
                    stringify!{[<$t:lower>]}
                }
                fn name_plural<'b>() -> &'b str {
                    stringify!{[<$t:lower s>]}
                }
            }
        }
    };
}

impl_qobuz_type!(Album, [Array<Track<WithoutExtra>>]);
impl_qobuz_type!(Track, [Album<WithoutExtra>]);
impl_qobuz_type!(
    Artist,
    [Array<Track<WithExtra>>, Array<Album<WithoutExtra>>]
);
impl_qobuz_type!(Playlist, [Array<Track<WithExtra>>]);

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
