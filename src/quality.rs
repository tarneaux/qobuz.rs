use core::fmt::{self, Display, Formatter};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default, clap::ValueEnum)]
#[serde(try_from = "u8")]
#[serde(into = "u8")]
pub enum Quality {
    Mp3,
    #[default]
    Cd,
    HiRes96,
    HiRes192,
}

impl Display for Quality {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mp3 => write!(f, "MP3"),
            Self::Cd => write!(f, "CD"),
            Self::HiRes96 => write!(f, "HiRes96"),
            Self::HiRes192 => write!(f, "HiRes192"),
        }
    }
}

impl TryFrom<u8> for Quality {
    type Error = InvalidQualityError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            5 => Ok(Self::Mp3),
            6 => Ok(Self::Cd),
            7 => Ok(Self::HiRes96),
            27 => Ok(Self::HiRes192),
            _ => Err(InvalidQualityError),
        }
    }
}

impl FromStr for Quality {
    type Err = InvalidQualityError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().replace('-', "").as_str() {
            "mp3" => Ok(Self::Mp3),
            "cd" => Ok(Self::Cd),
            "hires96" => Ok(Self::HiRes96),
            "hires192" => Ok(Self::HiRes192),
            v => Quality::try_from(v.parse::<u8>().map_err(|_| InvalidQualityError)?),
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid quality")]
pub struct InvalidQualityError;

impl From<Quality> for u8 {
    fn from(val: Quality) -> Self {
        match val {
            Quality::Mp3 => 5,
            Quality::Cd => 6,
            Quality::HiRes96 => 7,
            Quality::HiRes192 => 27,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum FileExtension {
    Mp3,
    Flac,
}

impl Display for FileExtension {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mp3 => write!(f, "mp3"),
            Self::Flac => write!(f, "flac"),
        }
    }
}

impl From<&Quality> for FileExtension {
    fn from(value: &Quality) -> Self {
        match value {
            Quality::Mp3 => Self::Mp3,
            Quality::Cd | Quality::HiRes96 | Quality::HiRes192 => Self::Flac,
        }
    }
}
