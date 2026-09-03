//! Reads HCI files in vendor text format.
//!
//! Each file contains one Mode2 measurement similar to the example below.
//!
//! ```ignore
//! event:
//! requester
//! 31 40 00 ...
//!
//! event:
//! requester
//! 32 40 00 ...
//!
//! [...]
//!
//! event:
//! reflector
//! 31 40 00 ...
//!
//! event:
//! reflector
//! 32 40 00 ...
//!
//! [...]
//! ```

use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use crate::event::hci_le_cs::subevent_result::{Origin, SubeventResultEvent};

/// Read lines of a text file.
///
/// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P: AsRef<Path>>(path: P) -> io::Result<io::Lines<io::BufReader<File>>> {
    let file = File::open(path)?;
    Ok(io::BufReader::new(file).lines())
}

/// Read a file in vendor HCI text format and generate subevent result events from them.
///
/// # Panics
///
/// Panics if the file cannot be read, a data token is not valid hexadecimal
/// or does not encode exactly one byte, or an event's bytes do not parse (see
/// [`crate::event::ParseError`]). Structural malformation of the text itself —
/// a data line with no preceding `event:` header, an unrecognized node label,
/// or a dangling label at the end of the file — is silently skipped, so a
/// truncated capture yields fewer events than the file holds.
pub fn read_file(path: &PathBuf) -> Vec<SubeventResultEvent> {
    /// The state of reading the input text file.
    enum ReadState {
        /// Wait for an event line.
        Event,
        /// Wait for the node specifier.
        Node,
        /// Wait for the data itself.
        Data(Origin),
    }

    let mut results = Vec::new();
    let mut read_state = ReadState::Event;

    for line in read_lines(path).unwrap() {
        let line = line.unwrap();
        let line = line.trim();

        read_state = match read_state {
            ReadState::Event => {
                if line == "event:" {
                    ReadState::Node
                } else {
                    ReadState::Event
                }
            }
            ReadState::Node => {
                if line == "reflector" {
                    ReadState::Data(Origin::Reflector)
                } else if line == "requester" {
                    ReadState::Data(Origin::Initiator)
                } else {
                    ReadState::Event
                }
            }
            ReadState::Data(node) => {
                let digits = line.split_whitespace();
                let values: Vec<u8> = digits
                    .into_iter()
                    .map(|token| {
                        let bytes = hex::decode(token).expect("vendor data token is valid hexadecimal");
                        assert_eq!(bytes.len(), 1, "vendor data token encodes exactly one byte: {token}");
                        bytes[0]
                    })
                    .collect();

                let result = SubeventResultEvent::try_from_with_origin(values.as_slice(), node).unwrap();
                results.push(result);

                ReadState::Event
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Writes a minimal vendor capture and returns its path.
    fn write_capture(dir: &Path, contents: &str) -> PathBuf {
        let path = dir.join("capture.txt");
        fs::write(&path, contents).expect("vendor capture is written");
        path
    }

    #[test]
    fn reads_labelled_events_in_order() {
        let dir = tempfile::tempdir().expect("temporary capture root is created");
        let path = write_capture(
            dir.path(),
            "event:\nrequester\n32 01 00 07 00 00 00 01 01 01 05 06 21 80 34 12 34 02\n\
             event:\nreflector\n32 02 00 07 00 00 00 01 00\n",
        );

        let events = read_file(&path);

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].origin, Origin::Initiator));
        assert_eq!(events[0].connection_handle, 1);
        assert!(matches!(events[1].origin, Origin::Reflector));
        assert_eq!(events[1].connection_handle, 2);
    }

    #[test]
    fn multi_byte_tokens_are_rejected_loudly() {
        let dir = tempfile::tempdir().expect("temporary capture root is created");
        // The token `0100` encodes two bytes; the old reader silently kept
        // its first byte and shifted every following header field.
        let path = write_capture(dir.path(), "event:\nrequester\n32 0100 00\n");

        let result = std::panic::catch_unwind(|| read_file(&path));

        let message = match result {
            Err(payload) => payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default(),
            Ok(_) => panic!("a glued multi-byte token must not be silently truncated"),
        };
        assert!(
            message.contains("encodes exactly one byte"),
            "the token-length guard must reject the token, not a downstream parse: {message}"
        );
    }
}
