use crate::{Album, Array, Composer, Track};
use serde::{Deserialize, Serialize};

pub trait PlaylistExtra {}
pub trait TrackExtra {}
pub trait AlbumExtra {}
pub trait ArtistExtra {}

impl TrackExtra for () {}
impl PlaylistExtra for () {}
impl AlbumExtra for () {}
impl ArtistExtra for () {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    pub tracks: Array<Track<()>>,
}

impl PlaylistExtra for Tracks {}
impl AlbumExtra for Tracks {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AlbumAndComposer {
    pub album: Album<()>,
    pub composer: Composer,
}

impl TrackExtra for AlbumAndComposer {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Albums {
    pub albums: Array<Album<()>>, // TODO: What is the extra here ?
}

impl ArtistExtra for Albums {}
