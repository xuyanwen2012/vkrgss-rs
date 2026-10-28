//! The entries of a frame schedule, the shape the input script and the states schedule share.
//!
//! A schedule is text: entries `frame:REST` separated by commas or line breaks. Blanks around
//! entries and around the frame are ignored, and so are empty entries such as the one after a
//! trailing comma. The frame is the text before the first `:`, a number from 0 that fits in a
//! `u64`; what follows the colon is read by each schedule.

/// An entry of a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Entry<'a> {
    pub frame: u64,
    /// What follows the first `:`, untrimmed.
    pub rest: &'a str,
    /// The whole entry, trimmed, for naming it in errors.
    pub text: &'a str,
}

/// Why an entry could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryError {
    MissingColon,
    InvalidFrame,
}

/// The entries of `text` in the order written, or the error of an entry with its text.
pub(crate) fn entries(text: &str) -> impl Iterator<Item = Result<Entry<'_>, (EntryError, &str)>> {
    let texts = text.split([',', '\n']).map(str::trim);
    texts.filter(|text| !text.is_empty()).map(|text| {
        let (frame, rest) = text
            .split_once(':')
            .ok_or((EntryError::MissingColon, text))?;
        let frame = frame
            .trim()
            .parse()
            .map_err(|_| (EntryError::InvalidFrame, text))?;
        Ok(Entry { frame, rest, text })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_split_trimmed_and_divided_at_the_first_colon() {
        let read: Vec<_> = entries(" , 2 : a:b ,\r\n\t\n,, 18446744073709551615:,").collect();
        assert_eq!(
            read,
            [
                Ok(Entry {
                    frame: 2,
                    rest: " a:b",
                    text: "2 : a:b"
                }),
                Ok(Entry {
                    frame: u64::MAX,
                    rest: "",
                    text: "18446744073709551615:"
                }),
            ]
        );
        assert_eq!(entries("").count(), 0);
    }

    #[test]
    fn a_malformed_entry_gives_its_error_and_text() {
        let first = |text| entries(text).next().unwrap().unwrap_err();
        assert_eq!(first(" 10 "), (EntryError::MissingColon, "10"));
        for text in ["ten:a", ":a", "-1:a", "1 2:a", "18446744073709551616:a"] {
            assert_eq!(first(text), (EntryError::InvalidFrame, text), "{text}");
        }
    }
}
