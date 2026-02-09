//! [`ExtraFlag`] definition.
//! See the [parent module documentation][super] to get started.

use super::{Album, Array, Artist, Playlist, Track};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

/// An extra flag, with an optional field of type `T`.
///
/// See the [parent module documentation][super] to get started.
///
/// Multiple types can be used with the same flag by constraining the same generic twice with a
/// different `T`.
pub trait ExtraFlag<T>: std::fmt::Debug {
    /// The type to use for the optional field's definition.
    type Extra: DeserializeOwned + Serialize + Eq + Clone + Debug;
}

/// An enabled extra flag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithExtra;
/// A disabled extra flag.
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

/// Empty struct for types with an [`ExtraFlag`] set to [`WithoutExtra`].
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

macro_rules! extra_utils {
    (
        $t:ident,
        [$( $extra_field:ident: $extra_type:ty ),+]
    ) => {
        impl $t<WithExtra> {
            #[doc = "Cast this item into its [`WithoutExtra`] variant."]
            #[must_use]
            pub fn without_extra(self) -> $t<WithoutExtra> {
                $t {
                    $( $extra_field: Empty, )+
                    ..self
                }
            }
        }
        impl $t<WithoutExtra> {
            #[doc = "Cast this item into its [`WithExtra`] variant."]
            #[must_use]
            pub fn with_extra(self, $( $extra_field: $extra_type ),+ ) -> $t<WithExtra> {
                $t {
                    $( $extra_field, )+
                    ..self
                }
            }
        }
    };
}

extra_utils!(Track, [album: Album<WithoutExtra>]);
extra_utils!(Album, [tracks: Array<Track<WithoutExtra>>]);
extra_utils!(Playlist, [tracks: Array<Track<WithExtra>>]);
extra_utils!(Artist, [tracks: Array<Track<WithExtra>>, albums: Array<Album<WithoutExtra>>]);
