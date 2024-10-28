use crate::{Array, Track};
use serde::{Deserialize, Serialize};

pub trait PlaylistExtra {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    pub tracks: Array<Track>,
}

impl PlaylistExtra for Tracks {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TracksCount {
    pub tracks_count: usize,
}

impl PlaylistExtra for TracksCount {}
