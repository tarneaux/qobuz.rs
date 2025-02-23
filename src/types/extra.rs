use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

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
