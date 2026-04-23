use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use ulid::Ulid;

pub const MIN_SHORT_ID_SUFFIX_LEN: usize = 8;
const ULID_LENGTH: usize = 26;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdError {
    #[error("invalid {kind} id `{value}`: expected prefix `{expected_prefix}_`")]
    InvalidPrefix {
        kind: &'static str,
        value: String,
        expected_prefix: &'static str,
    },
    #[error("invalid {kind} id `{value}`: suffix must be a ULID")]
    InvalidUlid { kind: &'static str, value: String },
    #[error(
        "invalid {kind} reference `{value}`: id suffix must be between {min} and {max} characters"
    )]
    InvalidReferenceLength {
        kind: &'static str,
        value: String,
        min: usize,
        max: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpaceId(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(String);

impl SpaceId {
    pub const PREFIX: &'static str = "spc";

    pub fn new() -> Self {
        Self(format!("{}_{}", Self::PREFIX, Ulid::new()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn short_id(&self) -> String {
        short_id(self.as_str(), Self::PREFIX)
    }

    pub fn matches_ref(&self, value: &str) -> Result<bool, IdError> {
        matches_ref(self.as_str(), Self::PREFIX, "space", value)
    }
}

impl TaskId {
    pub const PREFIX: &'static str = "tsk";

    pub fn new() -> Self {
        Self(format!("{}_{}", Self::PREFIX, Ulid::new()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn short_id(&self) -> String {
        short_id(self.as_str(), Self::PREFIX)
    }

    pub fn matches_ref(&self, value: &str) -> Result<bool, IdError> {
        matches_ref(self.as_str(), Self::PREFIX, "task", value)
    }
}

impl Default for SpaceId {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SpaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for SpaceId {
    type Error = IdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        validate_long_id(value, Self::PREFIX, "space")?;
        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<String> for SpaceId {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for TaskId {
    type Error = IdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        validate_long_id(value, Self::PREFIX, "task")?;
        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<String> for TaskId {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

fn short_id(value: &str, prefix: &str) -> String {
    let suffix = value
        .split_once('_')
        .map(|(_, suffix)| suffix)
        .unwrap_or(value);

    format!("{}_{}", prefix, &suffix[..MIN_SHORT_ID_SUFFIX_LEN])
}

fn validate_long_id(
    value: &str,
    expected_prefix: &'static str,
    kind: &'static str,
) -> Result<(), IdError> {
    let (prefix, suffix) = value
        .split_once('_')
        .ok_or_else(|| IdError::InvalidPrefix {
            kind,
            value: value.to_owned(),
            expected_prefix,
        })?;

    if prefix != expected_prefix {
        return Err(IdError::InvalidPrefix {
            kind,
            value: value.to_owned(),
            expected_prefix,
        });
    }

    if suffix.len() != ULID_LENGTH || Ulid::from_string(suffix).is_err() {
        return Err(IdError::InvalidUlid {
            kind,
            value: value.to_owned(),
        });
    }

    Ok(())
}

fn matches_ref(
    full_id: &str,
    expected_prefix: &'static str,
    kind: &'static str,
    reference: &str,
) -> Result<bool, IdError> {
    if reference == full_id {
        return Ok(true);
    }

    let (prefix, suffix) = reference
        .split_once('_')
        .ok_or_else(|| IdError::InvalidPrefix {
            kind,
            value: reference.to_owned(),
            expected_prefix,
        })?;

    if prefix != expected_prefix {
        return Err(IdError::InvalidPrefix {
            kind,
            value: reference.to_owned(),
            expected_prefix,
        });
    }

    if !(MIN_SHORT_ID_SUFFIX_LEN..=ULID_LENGTH).contains(&suffix.len()) {
        return Err(IdError::InvalidReferenceLength {
            kind,
            value: reference.to_owned(),
            min: MIN_SHORT_ID_SUFFIX_LEN,
            max: ULID_LENGTH,
        });
    }

    let (_, full_suffix) = full_id.split_once('_').expect("long ids are well formed");
    Ok(full_suffix.starts_with(suffix))
}
