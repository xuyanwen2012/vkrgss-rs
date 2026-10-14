//! Input recordings: the snapshots a guest consumed, stored for replay.
//!
//! A recording is an encoded stream (see [`crate::codec`]) of [`InputSnapshot`] values, one per
//! `Input.update`, in the order they were sampled.

use std::io::{Read, Write};

use thiserror::Error;

use crate::InputSnapshot;
use crate::codec::{self, DecodeError, EncodeError};

/// Why a recording could not be read or written.
#[derive(Debug, Error)]
pub enum RecordingError {
    #[error("input recording I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("input recording could not be encoded: {0}")]
    Encode(#[from] EncodeError),
    #[error("input recording could not be decoded: {0}")]
    Decode(#[from] DecodeError),
}

/// Writes `snapshots` as one recording.
pub fn write_recording<W: Write>(
    mut writer: W,
    snapshots: &[InputSnapshot],
) -> Result<(), RecordingError> {
    writer.write_all(&codec::encode_stream(snapshots)?)?;
    writer.flush()?;
    Ok(())
}

/// Reads a recording to its end and returns its snapshots in recorded order.
pub fn read_recording<R: Read>(mut reader: R) -> Result<Vec<InputSnapshot>, RecordingError> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(codec::decode_stream(&bytes)?)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Seek, SeekFrom};

    use super::*;
    use crate::codec::PROTOCOL_VERSION;
    use crate::{Button, FreezeRule, GuestToHost};

    /// A walk right, a confirm press held for three frames, then nothing.
    fn session() -> Vec<InputSnapshot> {
        let mut snapshots = vec![InputSnapshot::EMPTY; 4];
        snapshots.extend([InputSnapshot::EMPTY.with(Button::Right); 10]);
        snapshots.extend([InputSnapshot::EMPTY.with(Button::Right).with(Button::Shift); 5]);
        snapshots.extend([InputSnapshot::EMPTY.with(Button::C); 3]);
        snapshots.push(InputSnapshot::EMPTY);
        snapshots.extend(Button::ALL.map(|button| InputSnapshot::EMPTY.with(button)));
        snapshots
    }

    #[test]
    fn recording_round_trips_through_a_temp_file() {
        let recorded = session();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("session.vkrgss-input");

        write_recording(File::create(&path).unwrap(), &recorded).unwrap();
        let replayed = read_recording(File::open(&path).unwrap()).unwrap();

        assert_eq!(replayed, recorded);
    }

    #[test]
    fn empty_recording_round_trips() {
        let mut file = tempfile::tempfile().unwrap();
        write_recording(&mut file, &[]).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(read_recording(&mut file).unwrap(), vec![]);
    }

    #[test]
    fn recording_with_foreign_version_is_rejected() {
        let mut bytes = Vec::new();
        write_recording(&mut bytes, &session()).unwrap();
        bytes[4..6].copy_from_slice(&(PROTOCOL_VERSION + 1).to_le_bytes());
        assert!(matches!(
            read_recording(bytes.as_slice()),
            Err(RecordingError::Decode(
                DecodeError::UnsupportedVersion { .. }
            ))
        ));
    }

    #[test]
    fn message_stream_is_not_a_recording() {
        let bytes = codec::encode_stream(&[GuestToHost::Freeze {
            rule: FreezeRule::Frame,
        }])
        .unwrap();
        assert!(matches!(
            read_recording(bytes.as_slice()),
            Err(RecordingError::Decode(DecodeError::WrongStreamKind { .. }))
        ));
    }
}
