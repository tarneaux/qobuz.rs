//! Utility traits for Qobuz API types.

use super::{
    extra::{WithExtra, WithoutExtra},
    Album, Artist, Playlist, QobuzType, Track,
};

/// Item types that can be marked as a favorite.
pub trait Favoritable: ImplicitExtra {}

impl Favoritable for Track<WithExtra> {}
impl Favoritable for Album<WithoutExtra> {}
impl Favoritable for Artist<WithoutExtra> {}

/// Item types that are the root of an API query response.
pub trait RootEntity: QobuzType {
    /// The argument to be given to the API's "extra" argument when making a request for this item.
    fn extra_arg<'b>() -> &'b str
    where
        Self: Sized;
}

impl RootEntity for Track<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        ""
    }
}

impl RootEntity for Album<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        ""
    }
}

impl RootEntity for Artist<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        "tracks,albums"
    }
}

impl RootEntity for Playlist<WithExtra> {
    fn extra_arg<'b>() -> &'b str {
        "tracks"
    }
}

/// Types of Qobuz API responses when not setting the "extra" argument.
pub trait ImplicitExtra: QobuzType {}

impl ImplicitExtra for Track<WithExtra> {}
impl ImplicitExtra for Album<WithoutExtra> {}
impl ImplicitExtra for Artist<WithoutExtra> {}
impl ImplicitExtra for Playlist<WithExtra> {}

/// Downloadable items types.
pub trait Downloadable: QobuzType {}
impl Downloadable for Playlist<WithExtra> {}
impl Downloadable for Album<WithExtra> {}
impl Downloadable for Track<WithExtra> {}
