use super::{Album, Array, Artist, Playlist, Track};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

pub trait ExtraFlag<T>: std::fmt::Debug {
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

pub trait ExtraTwins {}
pub trait ExtraExtract {
    type EF;
}

macro_rules! extra_utils {
    (
        $t:ident,
        [$( $extra_field:ident: $extra_type:ty ),+]
    ) => {
        impl ExtraTwins for ($t<WithoutExtra>, $t<WithExtra>) {}
        impl ExtraTwins for ($t<WithExtra>, $t<WithoutExtra>) {}
        impl<EF> ExtraExtract for $t<EF>
        where
            $( EF: ExtraFlag<$extra_type>, )+
        {
            type EF = EF;
        }
        impl $t<WithExtra> {
            #[must_use]
            pub fn without_extra(self) -> $t<WithoutExtra> {
                $t {
                    $( $extra_field: Empty, )+
                    ..self
                }
            }
        }
        impl $t<WithoutExtra> {
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
