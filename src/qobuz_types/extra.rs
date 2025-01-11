use crate::{Album, Array, Artist, Composer, Playlist, Track};
use serde::{Deserialize, Serialize};

pub trait Extra: Serialize + for<'a> Deserialize<'a> {
    type Extra;
    fn extra_arg<'b>() -> Option<&'b str>;
}

impl Extra for Track<()> {
    // TODO: Querying tracks returns album and composer by default. Is this
    // the correct way to represent tracks that have been queried as an
    // extra themself (i.e. should they implement this trait ?)
    type Extra = ();
    fn extra_arg<'b>() -> Option<&'b str> {
        None
    }
}

impl Extra for Playlist<()> {
    type Extra = ();
    fn extra_arg<'b>() -> Option<&'b str> {
        None
    }
}

impl Extra for Album<()> {
    type Extra = ();
    fn extra_arg<'b>() -> Option<&'b str> {
        None
    }
}

impl Extra for Artist<()> {
    type Extra = ();
    fn extra_arg<'b>() -> Option<&'b str> {
        None
    }
}

impl Extra for Playlist<Tracks> {
    type Extra = Tracks;
    fn extra_arg<'b>() -> Option<&'b str> {
        Some("tracks")
    }
}

impl Extra for Album<Tracks> {
    type Extra = Tracks;
    fn extra_arg<'b>() -> Option<&'b str> {
        None
    }
}

impl Extra for Track<AlbumAndComposer> {
    type Extra = AlbumAndComposer;
    fn extra_arg<'b>() -> Option<&'b str> {
        None // Is returned by default when querying tracks
    }
}

impl Extra for Artist<Albums> {
    type Extra = Albums;
    fn extra_arg<'b>() -> Option<&'b str> {
        Some("albums")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tracks {
    pub tracks: Array<Track<()>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AlbumAndComposer {
    pub album: Album<()>,
    pub composer: Composer,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Albums {
    pub albums: Array<Album<()>>, // TODO: What is the extra here ?
}
