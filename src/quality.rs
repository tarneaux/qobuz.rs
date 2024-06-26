use core::fmt::{self, Display, Formatter};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(try_from = "u8")]
#[serde(into = "u8")]
pub enum Quality {
    Mp3,
    Cd,
    HiRes96,
    HiRes192,
}

impl Display for Quality {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mp3 => write!(f, "MP3 320"),
            Self::Cd => write!(f, "CD / Lossless"),
            Self::HiRes96 => write!(f, "Hi-Res 24-bit, up to 96 kHz"),
            Self::HiRes192 => write!(f, "Hi-Res 24-bit, up to 192 kHz"),
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
            v => Err(InvalidQualityError(v)),
        }
    }
}

#[derive(Debug)]
pub struct InvalidQualityError(u8);
impl Display for InvalidQualityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid quality: {}", self.0)
    }
}
impl Error for InvalidQualityError {}

impl Into<u8> for Quality {
    fn into(self) -> u8 {
        match self {
            Self::Mp3 => 5,
            Self::Cd => 6,
            Self::HiRes96 => 7,
            Self::HiRes192 => 27,
        }
    }
}
