use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

use super::{Album, Artist, Playlist, Track};

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

pub trait ExtraFlag<T> {
    type Extra: DeserializeOwned + Serialize + Eq + Clone + Debug;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithExtra;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithoutExtra;

impl<T> ExtraFlag<T> for WithExtra
where
    T: for<'a> Deserialize<'a> + Serialize + Eq + Clone + Debug,
{
    type Extra = T;
}
impl<T> ExtraFlag<T> for WithoutExtra {
    type Extra = Empty;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Empty;

impl Serialize for Empty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}

impl<'de> Deserialize<'de> for Empty {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self)
    }
}
