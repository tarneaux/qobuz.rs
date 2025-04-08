use super::{
    extra::{WithExtra, WithoutExtra},
    Album, Artist, Playlist, QobuzType, Track,
};

pub trait Favoritable: ImplicitExtra {}

impl Favoritable for Track<WithExtra> {}
impl Favoritable for Album<WithoutExtra> {}
impl Favoritable for Artist<WithoutExtra> {}

pub trait RootEntity: QobuzType {
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

pub trait ImplicitExtra: QobuzType {}

impl ImplicitExtra for Track<WithExtra> {}
impl ImplicitExtra for Album<WithoutExtra> {}
impl ImplicitExtra for Artist<WithoutExtra> {}
impl ImplicitExtra for Playlist<WithExtra> {}

pub trait Downloadable: QobuzType {}
impl Downloadable for Playlist<WithExtra> {}
impl Downloadable for Album<WithExtra> {}
impl Downloadable for Track<WithExtra> {}
