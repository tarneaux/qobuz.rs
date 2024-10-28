use crate::{Album, Array, Composer, Track};
use serde::{Deserialize, Serialize};

pub trait PlaylistExtra {}
pub trait TrackExtra {}
pub trait AlbumExtra {}

impl TrackExtra for () {}
impl PlaylistExtra for () {}
impl AlbumExtra for () {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    pub tracks: Array<Track<()>>,
}

impl PlaylistExtra for Tracks {}
impl AlbumExtra for Tracks {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TracksCount {
    pub tracks_count: usize,
}

impl PlaylistExtra for TracksCount {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AlbumAndComposer {
    pub album: Album<()>,
    pub composer: Composer,
}

impl TrackExtra for AlbumAndComposer {}
