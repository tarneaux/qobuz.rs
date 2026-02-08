//! Types returned by the Qobuz API.
//!
//! # Extra Flags
//!
//! The core Qobuz types have parts which may or may not be returned by the API depending on
//! whether or not the type is the root of the response or a child. For example, when querying a
//! track, the album it is from will also be returned, but this album's tracks will *not* be
//! returned.
//!
//! To prevent duplication of these types (one top-level and one lower-level version), they have
//! been implemented with a generic, named `EF` ("Extra Flag"). It can have one of two values :
//! [`WithExtra`] and [`WithoutExtra`]. WithExtra makes for a top-level struct containing it's
//! children's information (e.g an [`Album<WithExtra>`] will contain the album's tracks).
//!
//! For example, a function that needs a playlist with its tracks could look like this :
//!
//! ```
//! use qobuz::types::{Playlist, PlaylistExtra, Array, Track, extra::WithExtra};
//! fn get_playlist_tracks<'a>(playlist: &'a Playlist<WithExtra>) -> &Array<Track<WithExtra>> {
//!     &playlist.tracks
//! }
//! ```
//!
//! [`WithExtra`] and [`WithoutExtra`] are two empty structs implementing the [`ExtraFlag`] trait.
//! This trait takes a generic that corresponds to the optional field's type. To avoid extensive
//! type constraints when using these types, trait aliases have been created for each type of the
//! API that has an extra flag (namely [`Album`], [`Playlist`], [`Track`] and [`Artist`]). These
//! aliases are respectively called [`AlbumExtra`], [`PlaylistExtra`], [`TrackExtra`] and
//! [`ArtistExtra`].
//!
//! For example, a function with an [`Playlist`] argument that doesn't need to access the
//! playlist's tracks (and should therefore accept both [`Playlist<WithExtra>`] and
//! [`Playlist<WithoutExtra>`] arguments) can be written as follows :
//!
//! ```
//! use qobuz::types::{Playlist, PlaylistExtra};
//!
//! fn get_playlist_name<'a, EF: PlaylistExtra>(playlist: &'a Playlist<EF>) -> &'a str {
//!     &playlist.name
//! }
//! ```
//!
//! For convenience, [`Album`], [`Playlist`], [`Track`] and [`Artist`] have `without_extra` and
//! `with_extra` methods which allow converting between the [`WithExtra`] and [`WithoutExtra`]
//! variants.

pub mod extra;
pub mod formattable;
pub mod traits;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use extra::{ExtraFlag, WithExtra, WithoutExtra};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, time::Duration};
use url::Url;

/// Trait alias for [`Playlist`]'s [`ExtraFlag`] generic.
pub trait PlaylistExtra = ExtraFlag<Array<Track<WithExtra>>>;

/// A Qobuz playlist with its optional extra [`Track`]s.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist<EF: PlaylistExtra> {
    pub name: String,
    pub slug: String,
    pub owner: PlaylistOwner,
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

/// A [Playlist]'s owner.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaylistOwner {
    pub id: i64,
    pub name: String,
}

/// A queried array.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Array<T> {
    pub items: Vec<T>,
    pub limit: i64,
    pub offset: i64,
    pub total: i64,
}

/// Trait alias for [`Track`]'s [`ExtraFlag`] generic.
pub trait TrackExtra = ExtraFlag<Album<WithoutExtra>>;

/// A Qobuz track with its optional extra [`Album`].
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Track<EF: TrackExtra> {
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

impl<EF: TrackExtra> Display for Track<EF> {
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

/// Trait alias for [`Album`]'s [`ExtraFlag`] generic.
pub trait AlbumExtra = ExtraFlag<Array<Track<WithoutExtra>>>;

/// A Qobuz album with its optional extra [`Track`]s.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Album<EF: AlbumExtra> {
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

impl<EF: AlbumExtra> Display for Album<EF> {
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
    /// Get this album's tracks as if they had been queried on their own, without performing
    /// additional requests to the Qobuz API.
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

/// Trait alias for [`Artist`]'s [`ExtraFlag`] generic.
pub trait ArtistExtra = ExtraFlag<Array<Track<WithExtra>>> + ExtraFlag<Array<Album<WithoutExtra>>>;

/// An artist and their optional extra tracks and albums.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Artist<EF: ArtistExtra> {
    pub albums_count: u64,
    pub id: i64,
    pub image: Value,
    pub name: String,
    pub slug: String,
    pub tracks: <EF as ExtraFlag<Array<Track<WithExtra>>>>::Extra,
    pub albums: <EF as ExtraFlag<Array<Album<WithoutExtra>>>>::Extra,
}

impl<EF: ArtistExtra> Display for Artist<EF> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// An item's genre.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Genre {
    pub id: u64,
    pub name: String,
    // pub path: Vec<i64>,
    pub slug: String,
}

/// An image (for album covers).
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub large: String,
    pub small: String,
    pub thumbnail: String,
}

/// A label.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub albums_count: u64,
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub supplier_id: u64,
}

/// A composer.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Composer {
    pub id: u64,
    pub name: String,
}

/// A performer.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Performer {
    pub id: u64,
    pub name: String,
}

impl Display for Performer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// The genre of a playlist.
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

/// Trait implemented for the main, queriable Qobuz types, providing name information and depended
/// on by other traits.
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
