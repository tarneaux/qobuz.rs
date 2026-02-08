//! Runtime tools resembling Rust's `format!()`, checked at parsing time (i.e at runtime, but
//! before formatting).
//!
//! The only error that can occur is when parsing a [`Format`]. Using
//! [`format()`][Formattable::format] will never fail.
//!
//! # Example
//!
//! ```
//! use qobuz::runtime_formatter::{Format, Formattable, IllegalPlaceholderError};
//!
//! // A type to be formatted.
//! struct Car<'a> {
//!     production_year: u16,
//!     color: &'a str,
//!     max_speed: u16,
//! }
//!
//! impl Formattable for Car<'_> {
//!     type Placeholder = VehiclePlaceholder;
//!
//!     fn get_field(&self, field: &Self::Placeholder) -> String {
//!         match field {
//!             Self::Placeholder::Year => self.production_year.to_string(),
//!             Self::Placeholder::Color => self.color.to_string(),
//!             Self::Placeholder::MaxSpeed => format!("{}mph", self.max_speed),
//!             Self::Placeholder::Kind => "car".to_string(),
//!         }
//!     }
//! }
//!
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! enum VehiclePlaceholder {
//!     Year,
//!     Color,
//!     MaxSpeed,
//!     Kind,
//! }
//!
//! impl std::str::FromStr for VehiclePlaceholder {
//!     type Err = IllegalPlaceholderError;
//!     fn from_str(s: &str) -> Result<Self, Self::Err> {
//!         match s {
//!             "year" => Ok(Self::Year),
//!             "color" => Ok(Self::Color),
//!             "max_speed" => Ok(Self::MaxSpeed),
//!             "kind" => Ok(Self::Kind),
//!             _ => Err(IllegalPlaceholderError(s.to_string())),
//!         }
//!     }
//! }
//!
//! let my_car = Car {
//!     production_year: 1982,
//!     color: "red",
//!     max_speed: 200,
//! };
//!
//! // Errors only occur when compiling the format strings.
//! "{".parse::<Format<VehiclePlaceholder>>().unwrap_err();
//! "{bogus}".parse::<Format<VehiclePlaceholder>>().unwrap_err();
//! let format: Format<VehiclePlaceholder> = "{color} {kind} made in {year}, max speed \
//! {max_speed}".parse().unwrap();
//!
//! // format() always succeeds since the format string has already been compiled.
//! assert_eq!(my_car.format(&format), "red car made in 1982, max speed 200mph");
//! ```

use std::str::FromStr;
use thiserror::Error;

/// Trait implemented for formattable data types.
pub trait Formattable {
    type Placeholder: Placeholder;

    /// Format the value into a string using the provided [`Format`].
    fn format(&self, format: &Format<Self::Placeholder>) -> String {
        format
            .segments
            .iter()
            .map(|segment| match segment {
                FormatSegment::Placeholder(field) => self.get_field(field),
                FormatSegment::Literal(s) => s.to_string(),
            })
            .collect()
    }

    /// Get a field by its placeholder (i.e. format the placeholder segment).
    fn get_field(&self, field: &Self::Placeholder) -> String;
}

/// Traits needed for placeholders used in [`Format`].
pub trait Placeholder =
    FromStr<Err = IllegalPlaceholderError> + std::fmt::Debug + Clone + PartialEq + Eq;

/// A compiled format string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format<T: Placeholder> {
    segments: Vec<FormatSegment<T>>,
}

impl<T: Placeholder> FromStr for Format<T> {
    type Err = FormatParseError;

    /// Parse a format string.
    ///
    /// # Errors
    ///
    /// If the format string is missing closing braces, or contains bogus placeholders.
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

/// Error returned by [`Format::from_str()`] when an attempt is made to use a placeholder that does not exist.
#[derive(Debug, Clone, Error)]
#[error("Illegal placeholder: `{0}`")]
pub struct IllegalPlaceholderError(pub String);

/// Errors returned when parsing format strings.
#[derive(Debug, Clone, Error)]
pub enum FormatParseError {
    /// Illegal placeholder in the format string.
    #[error("Illegal placeholder error in format string: `{0}`")]
    IllegalPlaceHolderError(#[from] IllegalPlaceholderError),
    /// Missing closing brace in the format string.
    #[error("Missing closing brace in format string")]
    MissingClosingBrace,
}

/// A segment of a [`Format`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum FormatSegment<T> {
    /// A placeholder to be replaced by its value.
    Placeholder(T),
    /// A string literal.
    Literal(String),
}
