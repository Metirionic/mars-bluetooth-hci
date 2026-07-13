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
                let values: Vec<u8> = digits.into_iter().map(|x| hex::decode(x).unwrap()[0]).collect();

                let result = SubeventResultEvent::try_from_with_origin(values.as_slice(), node).unwrap();
                results.push(result);

                ReadState::Event
            }
        }
    }

    results
}
