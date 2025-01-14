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

impl Extra for Artist<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        "tracks,albums"
    }
}

impl Extra for Playlist<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        "tracks"
    }
}

// TODO: Rename
pub trait ExtraReturnedByDefault {}

impl ExtraReturnedByDefault for Track<WithExtra> {}
impl ExtraReturnedByDefault for Album<WithoutExtra> {}
impl ExtraReturnedByDefault for Artist<WithoutExtra> {}

// TODO: Upgrade, downgrade methods
// TODO: Change name ?
pub trait ExtraFlag {
    type Extra<T>;
}

// TODO: Rename, put in enum (enum probably won't work) ?
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithExtra;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithoutExtra;

impl ExtraFlag for WithExtra {
    type Extra<T> = T;
}
impl ExtraFlag for WithoutExtra {
    type Extra<T> = Empty;
}

// TODO: Is this the right way to do this ? Is there really no way to use () instead ?
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "Option<()>")] // NOTE: Will error out if the field does exist.
pub struct Empty;

impl From<Option<()>> for Empty {
    fn from(_: Option<()>) -> Self {
        Self
    }
}
