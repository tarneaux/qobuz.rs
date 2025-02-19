use crate::types::{
    extra::{ExtraFlag, WithoutExtra},
    Album, Array, Track,
};
use chrono::{Datelike, NaiveDate};
use id3::frame::Timestamp;
use std::path::Path;
use thiserror::Error;

pub async fn tag_track<EF1, EF2>(
    track: &Track<EF1>,
    path: &Path,
    album: &Album<EF2>,
) -> Result<(), TaggingError>
where
    EF1: ExtraFlag<Album<WithoutExtra>>,
    EF2: ExtraFlag<Array<Track<WithoutExtra>>>,
{
    let cover_raw = reqwest::get(album.image.large.clone())
        .await?
        .bytes()
        .await?;
    let cover = audiotags::Picture::new(&cover_raw, audiotags::MimeType::Jpeg);

    let mut tag = match audiotags::Tag::new().read_from_path(path) {
        Ok(v) => v,
        Err(e) => match e {
            audiotags::Error::Id3TagError(ref e2) if matches!(e2.kind, id3::ErrorKind::NoTag) => {
                // Id3 returns an error when there's no tag saved on the file yet, but then we can
                // just create a new empty tag.
                Box::new(audiotags::Id3v2Tag::new())
            }
            _ => {
                return Err(e.into());
            }
        },
    };
    tag.set_title(&track.title);
    tag.set_date(datetime_to_timestamp(track.release_date_original)?);
    tag.set_year(track.release_date_original.year());
    tag.set_album(audiotags::Album {
        title: &album.title,
        artist: Some(&album.artist.name),
        cover: Some(cover),
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

#[derive(Debug, Error)]
pub enum TaggingError {
    #[error("couldn't cast int type `{0}`")]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error("audiotags error `{0}`")]
    AudioTags(#[from] audiotags::Error),
    #[error("IO error `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("Reqwest error: `{0}`")]
    ReqwestError(#[from] reqwest::Error),
}
