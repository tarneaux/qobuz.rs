use crate::{
    types::{
        extra::{ExtraFlag, WithoutExtra},
        Album, Array, Track,
    },
    Quality,
};
use chrono::Datelike;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PathFormat {
    album_format: Format<AlbumPlaceholder>,
    track_format: Format<TrackPlaceholder>,
}

impl PathFormat {
    /// Formats an album directory path
    pub(super) fn get_album_dir<EF>(&self, album: &Album<EF>, quality: &Quality) -> String
    where
        EF: ExtraFlag<Array<Track<WithoutExtra>>>,
    {
        self.album_format.format(&AlbumInfo {
            artist: &album.artist.name,
            title: &album.title,
            year: album.release_date_original.year(),
            quality: quality.to_string().as_str(),
        })
    }

    /// Formats a track filename.
    pub(super) fn get_track_file_basename<EF>(&self, track: &Track<EF>) -> String
    where
        EF: ExtraFlag<Album<WithoutExtra>>,
    {
        self.track_format.format(&TrackInfo {
            track_number: track.track_number,
            title: &track.title,
        })
    }
}

impl Default for PathFormat {
    fn default() -> Self {
        Self {
            album_format: "{artist} - {title} ({year}) [{quality}]"
                .parse()
                .expect("Format is correct"),
            track_format: "{track_number}. {title}"
                .parse()
                .expect("Format is correct"),
        }
    }
}

/// Format struct for holding parsed format segments.
#[derive(Debug, Clone)]
pub struct Format<P: Placeholder> {
    segments: Vec<FormatSegment<P>>,
}

impl<P: Placeholder> std::fmt::Display for Format<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            self.segments
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<String>()
        )
    }
}

impl<T: Placeholder> FromStr for Format<T> {
    type Err = FormatParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut segments = Vec::new();
        let mut remaining = s;

        while let Some(start) = remaining.find('{') {
            // Push literal segment before '{'
            if start > 0 {
                segments.push(FormatSegment::Literal(remaining[..start].to_string()));
            }

            let end = remaining[start..]
                .find('}')
                .ok_or(FormatParseError::MissingClosingBrace)?
                + start;

            // Extract placeholder name
            let placeholder_str = &remaining[start + 1..end];
            let placeholder = T::from_str(placeholder_str)?;

            segments.push(FormatSegment::Placeholder(placeholder));

            remaining = &remaining[end + 1..]; // Move past '}'
        }

        // Push remaining literal text
        if !remaining.is_empty() {
            segments.push(FormatSegment::Literal(remaining.to_string()));
        }

        Ok(Self { segments })
    }
}

#[derive(Debug, Clone)]
pub enum FormatSegment<P: Placeholder> {
    Literal(String),
    Placeholder(P),
}

impl<P: Placeholder> std::fmt::Display for FormatSegment<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Literal(s) => write!(f, "{s}"),
            Self::Placeholder(p) => write!(f, "{p}"),
        }
    }
}

#[derive(Debug, Clone, Error)]
#[error("Illegal placeholder: `{0}`")]
pub struct IllegalPlaceholderError(String);

#[derive(Debug, Clone, Error)]
pub enum FormatParseError {
    #[error("Illegal placeholder error in format string: `{0}`")]
    IllegalPlaceHolderError(#[from] IllegalPlaceholderError),
    #[error("Missing closing brace in format string")]
    MissingClosingBrace,
}

pub trait Placeholder: FromStr<Err = IllegalPlaceholderError> + std::fmt::Display {}

macro_rules! impl_placeholder_and_info {
    ($type:ident, { $($field:ident: $ty:ty),+ $(,)? }) => {
        paste::paste! {
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub enum [<$type Placeholder>] {
                $( [< $field:camel >] ),+
            }

            impl Placeholder for [<$type Placeholder>] {}

            impl FromStr for [<$type Placeholder>] {
                type Err = IllegalPlaceholderError;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        $( stringify!($field) => Ok(Self::[< $field:camel >]), )+
                        _ => Err(IllegalPlaceholderError(s.to_string())),
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

            #[derive(Debug, Clone)]
            pub struct [<$type Info>]<'a> {
                $( pub $field: $ty ),+
            }

            impl<'a> Format<[<$type Placeholder>]> {
                #[must_use]
                pub fn format(&self, data: &[<$type Info>]<'a>) -> String {
                    self.segments.iter().map(|s| {
                        match s {
                            FormatSegment::Literal(s) => s.to_string(),
                            FormatSegment::Placeholder(ph) => {
                                let value = match ph {
                                    $( [<$type Placeholder>]::[< $field:camel >] => data.$field.to_string(), )+
                                };
                                value
                            }
                        }
                    }).collect()
                }
            }
        }
    }
}

impl_placeholder_and_info!(Track, {
    track_number: u64,
    title: &'a str,
});

impl_placeholder_and_info!(Album, {
    artist: &'a str,
    title: &'a str,
    year: i32,
    quality: &'a str,
});
