use crate::{extra::Extra, Album, Track};
use audiotags::{Album as TagAlbum, Tag};
use chrono::{Datelike, NaiveDate};
use id3::frame::Timestamp;
use std::{error::Error, fmt::Display};

pub async fn tag_tracks<E1, E2>(
    tracks_and_paths: Vec<(&Track<E1>, &str)>,
    album: &Album<E2>,
) -> Result<(), MultiTagError>
where
    Track<E1>: Extra + Sync,
    Album<E2>: Extra + Sync,
{
    let cover_raw = reqwest::get(album.image.large.clone())
        .await?
        .bytes()
        .await?;
    let cover = audiotags::Picture::new(&cover_raw, audiotags::MimeType::Jpeg);
    for (track, path) in tracks_and_paths {
        tag_track(track, path, album, cover.clone())?;
    }
    Ok(())
}

pub fn tag_track<E1, E2>(
    track: &Track<E1>,
    path: &str,
    album: &Album<E2>,
    album_cover: audiotags::Picture,
) -> Result<(), TaggingError>
where
    Track<E1>: Extra,
    Album<E2>: Extra,
{
    let mut tag = Tag::new().read_from_path(path)?;
    tag.set_title(&track.title);
    tag.set_date(datetime_to_timestamp(track.release_date_original)?);
    tag.set_year(track.release_date_original.year());
    tag.set_album(TagAlbum {
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
pub enum MultiTagError {
    TaggingError(TaggingError),
    ReqwestError(reqwest::Error),
}

impl Display for MultiTagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TaggingError(e) => write!(f, "Tagging error: {e}"),
            Self::ReqwestError(e) => write!(f, "Reqwest error: {e}"),
        }
    }
}

impl From<TaggingError> for MultiTagError {
    fn from(value: TaggingError) -> Self {
        Self::TaggingError(value)
    }
}

impl From<reqwest::Error> for MultiTagError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl Error for MultiTagError {}

#[derive(Debug)]
pub enum TaggingError {
    TryFromIntError(std::num::TryFromIntError),
    AudioTags(audiotags::Error),
}

impl Display for TaggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TryFromIntError(e) => write!(f, "Couldn't cast int type: {e}"),
            Self::AudioTags(e) => write!(f, "audiotags error: {e}"),
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
