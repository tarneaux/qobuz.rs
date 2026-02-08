use super::{Album, AlbumExtra, Track, TrackExtra};
use crate::placeholder_enum;
use crate::runtime_formatter::Formattable;
use chrono::Datelike;

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
