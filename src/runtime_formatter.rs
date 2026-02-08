use std::str::FromStr;
use thiserror::Error;

/// Traits implemented for formattable structs.
pub trait Formattable {
    type Placeholder: Placeholder;

    fn get_field(&self, field: &Self::Placeholder) -> String;
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
}

pub trait Placeholder =
    FromStr<Err = IllegalPlaceholderError> + std::fmt::Debug + Clone + PartialEq + Eq;

/// A compiled format
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format<T: Placeholder> {
    segments: Vec<FormatSegment<T>>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum FormatSegment<T> {
    Placeholder(T),
    Literal(String),
}

#[derive(Debug, Clone, Error)]
#[error("Illegal placeholder: `{0}`")]
pub struct IllegalPlaceholderError(pub String);

#[derive(Debug, Clone, Error)]
pub enum FormatParseError {
    #[error("Illegal placeholder error in format string: `{0}`")]
    IllegalPlaceHolderError(#[from] IllegalPlaceholderError),
    #[error("Missing closing brace in format string")]
    MissingClosingBrace,
}

#[macro_export]
macro_rules! placeholder_enum {
    ($type:ident, [ $($field:ident),+ $(,)? ]) => {
        paste::paste! {
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
