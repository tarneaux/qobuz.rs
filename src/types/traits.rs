use crate::types::{
    extra::{ImplicitExtra, WithExtra, WithoutExtra},
    Album, Artist, Track,
};

pub trait Favoritable: ImplicitExtra {}

impl Favoritable for Track<WithExtra> {}
impl Favoritable for Album<WithoutExtra> {}
impl Favoritable for Artist<WithoutExtra> {}
