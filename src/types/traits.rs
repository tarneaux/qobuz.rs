use super::{
    extra::{WithExtra, WithoutExtra},
    Album, Artist, Playlist, Track,
};

pub trait Favoritable: ImplicitExtra {}

impl Favoritable for Track<WithExtra> {}
impl Favoritable for Album<WithoutExtra> {}
impl Favoritable for Artist<WithoutExtra> {}

pub trait RootEntity {
    fn extra_arg<'b>() -> &'b str;
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

pub trait ImplicitExtra {}

impl ImplicitExtra for Track<WithExtra> {}
impl ImplicitExtra for Album<WithoutExtra> {}
impl ImplicitExtra for Artist<WithoutExtra> {}
impl ImplicitExtra for Playlist<WithExtra> {}
