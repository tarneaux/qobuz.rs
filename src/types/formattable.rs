//! Tools for Qobuz API types that can be formatted with
//! [runtime_formatter][crate::runtime_formatter].

use super::{Album, AlbumExtra, Track, TrackExtra};
use crate::runtime_formatter::Formattable;
use chrono::Datelike;

macro_rules! placeholder_enum {
    ($type:ident, [ $($field:ident),+ $(,)? ]) => {
        paste::paste! {
            #[doc = concat!("Format placeholder for [`", stringify!($type), "`].")]
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub enum [<$type Placeholder>] {
                $( [< $field:camel >] ),+
            }

            impl std::str::FromStr for [<$type Placeholder>] {
                type Err = $crate::runtime_formatter::IllegalPlaceholderError;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        $( stringify!($field) => Ok(Self::[< $field:camel >]), )+
                        _ => Err($crate::runtime_formatter::IllegalPlaceholderError(s.to_string())),
                    }
                }
            }

            impl std::fmt::Display for [<$type Placeholder>] {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                    match self {
                        $( Self::[< $field:camel >] => write!(f, stringify!($field)), )+
                    }
                }
            }
        }
    }
}

impl<EF: AlbumExtra> Formattable for Album<EF> {
    type Placeholder = AlbumPlaceholder;

    fn get_field(&self, field: &Self::Placeholder) -> String {
        match field {
            AlbumPlaceholder::Year => self.release_date_original.year().to_string(),
            AlbumPlaceholder::Title => self.title.clone(),
            AlbumPlaceholder::Artist => self.artist.name.clone(),
        }
    }
}

placeholder_enum!(Album, [title, year, artist]);

impl<EF: TrackExtra> Formattable for Track<EF> {
    type Placeholder = TrackPlaceholder;

    fn get_field(&self, field: &Self::Placeholder) -> String {
        match field {
            TrackPlaceholder::Title => self.title.clone(),
            TrackPlaceholder::TrackNumber => self.track_number.to_string(),
            TrackPlaceholder::MediaNumber => self.media_number.to_string(),
        }
    }
}

placeholder_enum!(Track, [track_number, title, media_number]);
