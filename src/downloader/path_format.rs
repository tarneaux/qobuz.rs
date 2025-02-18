use tera::{Context, Tera};

use crate::types::{
    extra::{ExtraFlag, WithExtra, WithoutExtra},
    Album, Array, Playlist, Track,
};

#[derive(Debug, Clone)]
pub struct PathFormat {
    pub album_format: String,
    pub track_format: String,
    pub m3u_format: String,
}

impl PathFormat {
    pub fn get_album_dir<EF>(&self, album: &Album<EF>) -> Result<String, tera::Error>
    where
        EF: ExtraFlag<Array<Track<WithoutExtra>>>,
    {
        let context = Context::from_serialize(album)?;
        apply_template(&self.album_format, &context)
    }

    pub fn get_track_file_basename<EF>(&self, track: &Track<EF>) -> Result<String, tera::Error>
    where
        EF: ExtraFlag<Album<WithoutExtra>>,
    {
        let context = Context::from_serialize(track)?;
        apply_template(&self.track_format, &context)
    }

    pub fn get_m3u_file_basename(
        &self,
        playlist: &Playlist<WithExtra>,
    ) -> Result<String, tera::Error> {
        let context = Context::from_serialize(playlist)?;
        apply_template(&self.m3u_format, &context)
    }
}

impl Default for PathFormat {
    fn default() -> Self {
        Self {
            album_format: "{{ artist.name }} - {{ title }}".to_string(),
            track_format: "{{ track_number }}. {{ title }}".to_string(),
            m3u_format: "{{ name }}".to_string(),
        }
    }
}

fn apply_template(template: &str, context: &Context) -> Result<String, tera::Error> {
    let mut tera = Tera::default();
    tera.render_str(template, context)
}
