//! Serialization and deserialization components.
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use postcard::{from_bytes, to_allocvec};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Holds generic serialized data in binary form
#[derive(Serialize, Deserialize, Default)]
pub struct BinaryData {
    /// Sampling timestamps.
    timestamps: Vec<DateTime<Utc>>,
    /// Binary data vectors.
    raw: Vec<Vec<u8>>,
}

impl BinaryData {
    /// Create an empty instance.
    pub fn new() -> Self {
        Self {
            timestamps: Vec::new(),
            raw: Vec::new(),
        }
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
    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        from_bytes(buf).ok()
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
