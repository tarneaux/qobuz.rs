use crate::{Album, Array, Composer, Track};
use serde::{Deserialize, Serialize};

pub trait Extra {
    fn extra_arg<'b>() -> Option<&'b str>;
}

pub trait PlaylistExtra: Extra {}
pub trait TrackExtra: Extra {}
pub trait AlbumExtra: Extra {}
pub trait ArtistExtra: Extra {}

impl Extra for () {
    fn extra_arg<'b>() -> Option<&'b str> {
        None
    }
}

impl TrackExtra for () {}
impl PlaylistExtra for () {}
impl AlbumExtra for () {}
impl ArtistExtra for () {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    pub tracks: Array<Track<()>>,
}

impl Extra for Tracks {
    fn extra_arg<'b>() -> Option<&'b str> {
        Some("tracks")
    }
}

impl PlaylistExtra for Tracks {}
impl AlbumExtra for Tracks {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AlbumAndComposer {
    pub album: Album<()>,
    pub composer: Composer,
}

impl Extra for AlbumAndComposer {
    fn extra_arg<'b>() -> Option<&'b str> {
        None // Is returned by default when querying tracks
    }
}

impl TrackExtra for AlbumAndComposer {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Albums {
    pub albums: Array<Album<()>>, // TODO: What is the extra here ?
}

impl Extra for Albums {
    fn extra_arg<'b>() -> Option<&'b str> {
        Some("albums")
    }
}

impl ArtistExtra for Albums {}
