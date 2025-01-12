use crate::{extra::Extra, Album, Track};
use chrono::{Datelike, NaiveDate};
use id3::frame::Timestamp;
use std::{error::Error, fmt::Display, path::Path};

pub fn tag_track<E1, E2>(
    track: &Track<E1>,
    path: &Path,
    album: &Album<E2>,
    album_cover: audiotags::Picture,
) -> Result<(), TaggingError>
where
    Track<E1>: Extra,
    Album<E2>: Extra,
{
    let mut tag = audiotags::Tag::new().read_from_path(path)?;
    tag.set_title(&track.title);
    tag.set_date(datetime_to_timestamp(track.release_date_original)?);
    tag.set_year(track.release_date_original.year());
    tag.set_album(audiotags::Album {
        title: &album.title,
        artist: Some(&album.artist.name),
        cover: Some(album_cover),
    });
    tag.set_disc((
        track.media_number.try_into()?,
        album.media_count.try_into()?,
    ));
    tag.set_track_number(track.track_number.try_into()?);
    tag.set_artist(&album.artist.name);
    tag.set_genre(&album.genre.name);

    tag.write_to_path(path)?;
    Ok(())
}

fn datetime_to_timestamp(dt: NaiveDate) -> Result<Timestamp, std::num::TryFromIntError> {
    Ok(Timestamp {
        day: Some(dt.day0().try_into()?),
        month: Some(dt.month0().try_into()?),
        year: dt.year_ce().1.try_into()?,
        hour: None,
        minute: None,
        second: None,
    })
}

#[derive(Debug)]
pub enum TaggingError {
    TryFromIntError(std::num::TryFromIntError),
    AudioTags(audiotags::Error),
    IoError(std::io::Error),
}

impl Display for TaggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TryFromIntError(e) => write!(f, "Couldn't cast int type: {e}"),
            Self::AudioTags(e) => write!(f, "audiotags error: {e}"),
            Self::IoError(e) => write!(f, "io error: {e}"),
        }
    }
}

impl Error for TaggingError {}

impl From<audiotags::Error> for TaggingError {
    fn from(value: audiotags::Error) -> Self {
        Self::AudioTags(value)
    }
}

impl From<std::num::TryFromIntError> for TaggingError {
    fn from(value: std::num::TryFromIntError) -> Self {
        Self::TryFromIntError(value)
    }
}

impl From<std::io::Error> for TaggingError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}
