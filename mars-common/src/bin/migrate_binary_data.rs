//! Migrate BinaryData .bin files from format version 0 (no version field)
//! to format version 1 (with version field).
//!
//! Old format: postcard({timestamps: Vec<DateTime<Utc>>, raw: Vec<Vec<u8>>})
//! New format: postcard({version: u8, timestamps: Vec<DateTime<Utc>>, raw: Vec<Vec<u8>>})
//!
//! Usage:
//!   cargo run -p mars-common --bin migrate_binary_data -- <file.bin> [output.bin]
//!
//! If output is not specified, the file is migrated in-place (with a .bak backup).

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};

const BINARY_FORMAT_VERSION: u8 = 1;

#[derive(Serialize, Deserialize)]
struct LegacyBinaryData {
    timestamps: Vec<DateTime<Utc>>,
    raw: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct BinaryData {
    version: u8,
    timestamps: Vec<DateTime<Utc>>,
    raw: Vec<Vec<u8>>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.bin> [output.bin]", args[0]);
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let output_path = if args.len() > 2 {
        PathBuf::from(&args[3])
    } else {
        input_path.clone()
    };

    let buf = fs::read(&input_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {e}", input_path.display());
        std::process::exit(1);
    });

    // Try new format first
    if let Ok(data) = from_bytes::<BinaryData>(&buf) {
        if data.version == BINARY_FORMAT_VERSION {
            println!(
                "{} is already format version {}, skipping.",
                input_path.display(),
                data.version
            );
            return;
        }
        // Version mismatch — likely a legacy file where the first byte was
        // interpreted as version. Fall through to legacy deserialization.
    }

    // Try legacy format
    let legacy: LegacyBinaryData = match from_bytes(&buf) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to deserialize {}: {e}", input_path.display());
            std::process::exit(1);
        }
    };

    let migrated = BinaryData {
        version: BINARY_FORMAT_VERSION,
        timestamps: legacy.timestamps,
        raw: legacy.raw,
    };

    let output_buf = to_allocvec(&migrated).expect("Serialization should not fail");

    // Backup original if migrating in-place
    if output_path == input_path {
        let backup_path = input_path.with_extension("bin.bak");
        fs::copy(&input_path, &backup_path).unwrap_or_else(|e| {
            eprintln!("Failed to create backup {}: {e}", backup_path.display());
            std::process::exit(1);
        });
        println!("Backup created at {}", backup_path.display());
    }

    fs::write(&output_path, &output_buf).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {e}", output_path.display());
        std::process::exit(1);
    });

    println!(
        "Migrated {} -> {} (version 0 -> {})",
        input_path.display(),
        output_path.display(),
        BINARY_FORMAT_VERSION
    );
}