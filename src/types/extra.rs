use serde::{Deserialize, Serialize};

use super::{Album, Artist, Playlist, Track};

// TODO: Rename and move this?
// TODO: More possible extra's ?
// TODO: Make this an attribute directly on types, that is applied only if needed (?)
// TODO: OptionalExtra so that we can also query items without deserializing their extra if we
// don't want them
// TODO: Rename to something like QueryItself to reflect what the trait is actually used for
// (i.e. get_item)
// NOTE: Other possible extra's can be gotten by using a nonexistent extra and showing the text of
// the response
pub trait Extra {
    fn extra_arg<'b>() -> &'b str;
}

impl Extra for Track<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        ""
    }
}

impl Extra for Album<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        ""
    }
}

/// FIX: If we keep this implementation with two extra flags, there should be more than just this
/// implementation
impl Extra for Artist<WithExtra, WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        "tracks,albums"
    }
}

impl Extra for Playlist<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        "tracks"
    }
}

// TODO: Upgrade, downgrade methods
// TODO: Change name ?
// TODO: Avoid having to use Type<EF> where ... and replace it with a trait that directly has the
// correct type ?
pub trait ExtraFlag<T> {
    type Extra: Serialize + for<'a> Deserialize<'a> + Sync;
}

// TODO: Rename, put in enum (enum probably won't work) ?
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithExtra;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithoutExtra;

impl<T> ExtraFlag<T> for WithExtra
where
    T: Serialize + for<'a> Deserialize<'a> + Sync,
{
    type Extra = T;
}
impl<T> ExtraFlag<T> for WithoutExtra {
    type Extra = Empty;
}

// TODO: Is this the right way to do this ? Is there really no way to use () instead ?
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "Option<()>")] // Very ugly, will error out if the field does exist.
pub struct Empty;

impl From<Option<()>> for Empty {
    fn from(_: Option<()>) -> Self {
        Self
    }
}

// pub trait TupleExtract {
// type T1;
// type T2;
// }

// impl<T1, T2> TupleExtract for (T1, T2) {
// type T1 = T1;
// type T2 = T2;
// }

// impl TupleExtract for () {
// type T1 = ();
// type T2 = ();
// }

// #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
// #[serde(rename_all = "camelCase")]
// pub struct Tracks {
//     pub tracks: Array<Track<()>>,
// }

// #[derive(serialize, deserialize, debug, clone, partialeq, eq)]
// pub struct AlbumAndComposer {
//     pub album: Album<()>,
//     pub composer: Option<Composer>,
// }

// #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
// pub struct AlbumsAndTracks {
//     pub albums: Array<Album<()>>, // TODO: What is the extra here ?
//     pub tracks: Array<Track<()>>,
// }
