//! The differential helper: one message stream through the CPU reference compositor and the
//! Vulkan backend, compared step by step.

use vkxp_compose_ref as reference;
use vkxp_proto::GuestToHost;

use super::harness::TestCompositor;
use crate::{BitmapError, Frame, SlotState, VkError};

/// The error this backend must return where the reference returns `error`: the same kind with
/// the same indices, ids and values.
pub(crate) fn expected_error(error: &reference::ComposeError) -> VkError {
    use reference::ComposeError as E;
    match error.clone() {
        E::UnsupportedOp { kind, index } => VkError::UnsupportedOp { kind, index },
        E::InvalidOpBitmap { index, source } => VkError::InvalidOpBitmap {
            index,
            source: expected_bitmap_error(source),
        },
        E::SourceOutOfBounds {
            index,
            bitmap,
            src,
            width,
            height,
        } => VkError::SourceOutOfBounds {
            index,
            bitmap,
            src,
            width,
            height,
        },
        E::InvalidDelta { index, source } => VkError::InvalidDelta {
            index,
            source: expected_bitmap_error(source),
        },
        E::InvalidLogicalSize { size, limit } => VkError::InvalidLogicalSize { size, limit },
    }
}

fn expected_bitmap_error(error: reference::BitmapError) -> BitmapError {
    use reference::BitmapError as E;
    match error {
        E::InvalidReference { id, slot } => BitmapError::InvalidReference {
            id,
            slot: match slot {
                reference::SlotState::Empty => SlotState::Empty,
                reference::SlotState::Occupied { generation } => SlotState::Occupied { generation },
            },
        },
        E::SlotOccupied { id, occupant } => BitmapError::SlotOccupied { id, occupant },
        E::TooLarge {
            id,
            width,
            height,
            limit,
        } => BitmapError::TooLarge {
            id,
            width,
            height,
            limit,
        },
        E::UpdateOutOfBounds {
            id,
            rect,
            width,
            height,
        } => BitmapError::UpdateOutOfBounds {
            id,
            rect,
            width,
            height,
        },
        E::UpdateLengthMismatch {
            id,
            expected,
            found,
        } => BitmapError::UpdateLengthMismatch {
            id,
            expected,
            found,
        },
    }
}

/// Describes the first difference between two frames, or returns `None` when they are equal
/// byte for byte.
pub(crate) fn frame_difference(expected: &reference::Frame, actual: &Frame) -> Option<String> {
    if expected.size != actual.size {
        return Some(format!(
            "frame size: reference {:?}, vulkan {:?}",
            expected.size, actual.size
        ));
    }
    if expected.rgba.len() != actual.rgba.len() {
        return Some(format!(
            "frame bytes: reference {}, vulkan {}",
            expected.rgba.len(),
            actual.rgba.len()
        ));
    }
    let pixels = expected
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(actual.rgba.as_chunks::<4>().0);
    let different = pixels.clone().filter(|(a, b)| a != b).count();
    let (index, (reference, vulkan)) = pixels.enumerate().find(|(_, (a, b))| a != b)?;
    let width = expected.size.width as usize;
    Some(format!(
        "{different} pixel(s) differ; first at ({}, {}): reference {reference:?}, vulkan \
         {vulkan:?}",
        index % width,
        index / width
    ))
}

/// The two backends, fed the same messages.
pub(crate) struct Pair {
    reference: reference::Compositor,
    vulkan: TestCompositor,
    /// The last frame both produced, which a failed packet must leave in place.
    last: Option<reference::Frame>,
    steps: usize,
}

impl Pair {
    pub(crate) fn new() -> Pair {
        Pair {
            reference: reference::Compositor::new(),
            vulkan: TestCompositor::new(),
            last: None,
            steps: 0,
        }
    }

    /// Gives one message to both backends. They must both produce a frame with equal bytes,
    /// both produce none, or both fail with the same error. Returns the reference's result.
    pub(crate) fn step(
        &mut self,
        message: &GuestToHost,
    ) -> Result<Result<Option<reference::Frame>, reference::ComposeError>, String> {
        let step = self.steps;
        self.steps += 1;
        let expected = self.reference.handle(message);
        let actual = self.vulkan.handle(message);
        match (&expected, &actual) {
            (Ok(Some(frame)), Ok(true)) => {
                let rendered = self
                    .vulkan
                    .read_frame()
                    .map_err(|error| format!("step {step}: readback failed: {error}"))?
                    .ok_or_else(|| format!("step {step}: vulkan rendered but has no frame"))?;
                if let Some(difference) = frame_difference(frame, &rendered) {
                    return Err(format!("step {step}: {difference}"));
                }
                self.last = Some(frame.clone());
            }
            (Ok(None), Ok(false)) => {}
            (Err(error), Err(actual)) => {
                let wanted = expected_error(error);
                if *actual != wanted {
                    return Err(format!(
                        "step {step}: reference failed with `{wanted}`, vulkan with `{actual}`"
                    ));
                }
                // No frame for the failed packet: the target still holds the previous one.
                let kept = self
                    .vulkan
                    .read_frame()
                    .map_err(|error| format!("step {step}: readback failed: {error}"))?;
                let unchanged = match (&self.last, &kept) {
                    (Some(last), Some(kept)) => frame_difference(last, kept).is_none(),
                    (None, None) => true,
                    _ => false,
                };
                if !unchanged {
                    return Err(format!(
                        "step {step}: a failed packet changed the frame of the vulkan backend"
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "step {step}: reference returned {:?}, vulkan {actual:?}",
                    expected.as_ref().map(|frame| frame.is_some())
                ));
            }
        }
        Ok(expected)
    }
}

/// Runs a whole stream through both backends. An error describes the first step at which they
/// disagree.
pub(crate) fn compare_stream<'a>(
    messages: impl IntoIterator<Item = &'a GuestToHost>,
) -> Result<(), String> {
    let mut pair = Pair::new();
    for message in messages {
        let _reference_result = pair.step(message)?;
    }
    Ok(())
}

/// Panics unless both backends agree on every step of the stream.
pub(crate) fn assert_stream_matches<'a>(messages: impl IntoIterator<Item = &'a GuestToHost>) {
    if let Err(difference) = compare_stream(messages) {
        panic!("backends disagree: {difference}");
    }
}
