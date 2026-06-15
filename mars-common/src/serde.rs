//! Serialization and deserialization components.
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use postcard::{from_bytes, to_allocvec};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Wire format version for BinaryData serialization.
///
/// Bump this when the BinaryData struct layout changes in a way that
/// breaks postcard deserialization of previously recorded .bin files.
/// This includes: adding/removing/reordering fields, changing field types.
/// Do NOT bump for: new methods, refactors that don't change the struct,
/// or changes to code that doesn't affect the serialized format.
pub const BINARY_FORMAT_VERSION: u8 = 1;

/// Errors that can occur when deserializing BinaryData.
#[derive(Debug, thiserror::Error)]
pub enum BinaryDataError {
    /// The recording file uses an incompatible format version.
    #[error("Unsupported recording format version: expected {expected}, got {got}")]
    VersionMismatch {
        /// The expected format version.
        expected: u8,
        /// The format version found in the file.
        got: u8,
    },
    /// The recording file could not be deserialized.
    #[error("Failed to deserialize recording data")]
    DeserializationError(#[from] postcard::Error),
}

/// Holds generic serialized data in binary form
#[derive(Serialize, Deserialize)]
pub struct BinaryData {
    version: u8,
    /// Sampling timestamps.
    timestamps: Vec<DateTime<Utc>>,
    /// Binary data vectors.
    raw: Vec<Vec<u8>>,
}

impl Default for BinaryData {
    fn default() -> Self {
        Self::new()
    }
}

impl BinaryData {
    /// Create an empty instance.
    pub fn new() -> Self {
        Self {
            version: BINARY_FORMAT_VERSION,
            timestamps: Vec::new(),
            raw: Vec::new(),
        }
    }

    /// The format version of this recording.
    pub fn version(&self) -> u8 {
        self.version
    }

    /// The length of captured binary data.
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Check if binary data is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Push new data to the base data structure.
    pub fn push(&mut self, timestamp: DateTime<Utc>, raw_data: &[u8]) {
        self.timestamps.push(timestamp);
        self.raw.push(raw_data.to_vec())
    }

    /// Get the sampling timestamps.
    pub fn timestamps(&self) -> &[DateTime<Utc>] {
        self.timestamps.as_slice()
    }

    /// Get the raw data.
    pub fn raw(&self) -> &[Vec<u8>] {
        self.raw.as_slice()
    }

    /// Deserialize from a binary postcard stream.
    ///
    /// Returns an error if the format version doesn't match
    /// [`BINARY_FORMAT_VERSION`] or if the data is corrupt.
    pub fn deserialize(buf: &[u8]) -> Result<Self, BinaryDataError> {
        let data: Self = from_bytes(buf)?;
        if data.version != BINARY_FORMAT_VERSION {
            return Err(BinaryDataError::VersionMismatch {
                expected: BINARY_FORMAT_VERSION,
                got: data.version,
            });
        }
        Ok(data)
    }

    /// Serialize to a binary stream with postcard.
    pub fn serialize(&self) -> Vec<u8> {
        to_allocvec(self).unwrap()
    }
}

/// Serialize a config value to TOML and store it at the provided path.
pub fn store_config<T: Serialize>(config: &T, file_path: &PathBuf) {
    let toml_string = match toml::to_string_pretty(config) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to serialize config: {}", e);
            return;
        }
    };

    match File::create(file_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(toml_string.as_bytes()) {
                error!("Error writing config file: {}", e);
            }
        }
        Err(_) => error!("Error while opening file at '{:?}'", file_path),
    }
}

/// Load and deserialize a TOML config value from the provided path.
pub fn load_config<T: DeserializeOwned>(file_path: &PathBuf) -> Option<T> {
    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => {
            error!("Error while opening file at '{:?}'", file_path);
            return None;
        }
    };

    let mut buf = String::new();
    if let Err(e) = file.read_to_string(&mut buf) {
        error!("Error reading config file: {}", e);
        return None;
    }

    match toml::from_str::<T>(&buf) {
        Ok(config) => Some(config),
        Err(e) => {
            error!("Invalid config file: {}", e);
            None
        }
    }
}