//! `log_buffer` provides a way to record and extract logs without allocation.
//! The [LogBuffer](struct.LogBuffer.html) achieves this by providing a ring
//! buffer, similar to a *nix _dmesg_ facility.
//!
//! # Usage example
//!
//! ```
//! use std::fmt::Write;
//!
//! let mut dmesg = log_buffer::LogBuffer::new([0; 16]);
//! write!(dmesg, "\nfirst\n").unwrap();
//! write!(dmesg, "second\n").unwrap();
//! write!(dmesg, "third\n").unwrap();
//!
//! assert_eq!(dmesg.extract(),
//!            "st\nsecond\nthird\n");
//! assert_eq!(dmesg.extract_lines().collect::<Vec<_>>(),
//!            vec!["second", "third"]);
//! ```
//!
//! # Choices of backing storage
//!
//! Backed by an array:
//!
//! ```
//! let mut dmesg = log_buffer::LogBuffer::new([0; 16]);
//! ```
//!
//! Backed by a mutable slice:
//!
//! ```
//! let mut storage = [0; 16];
//! let mut dmesg = log_buffer::LogBuffer::new(&mut storage);
//! ```
//!
//! Backed by a vector:
//!
//! ```
//! let mut dmesg = log_buffer::LogBuffer::new(vec![0; 16]);
//! ```

#![no_std]

/// A ring buffer that stores UTF-8 text.
///
/// Anything that implements `AsMut<[u8]>` can be used for backing storage;
/// e.g. `[u8; N]`, `Vec<[u8]>`, `Box<[u8]>`.
#[derive(Debug)]
pub struct LogBuffer<T: AsRef<[u8]> + AsMut<[u8]>> {
    buffer:   T,
    position: usize
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> LogBuffer<T> {
    /// Creates a new ring buffer, backed by `storage`.
    ///
    /// The buffer is cleared after creation.
    pub fn new(storage: T) -> LogBuffer<T> {
        let mut buffer = LogBuffer { buffer: storage, position: 0 };
        buffer.clear();
        buffer
    }

    /// Creates a new ring buffer, backed by `storage`.
    ///
    /// The buffer is *not* cleared after creation, and contains whatever is in `storage`.
    /// The `clear()` method should be called before use.
    /// However, this function can be used in a static initializer.
    pub const fn uninitialized(storage: T) -> LogBuffer<T> {
        LogBuffer { buffer: storage, position: 0 }
    }

    /// Clears the buffer.
    ///
    /// Only the text written after clearing will be read out by a future extraction.
    ///
    /// This function takes O(n) time where n is buffer length.
    pub fn clear(&mut self) {
        self.position = 0;
        for b in self.buffer.as_mut().iter_mut() {
            // Any non-leading UTF-8 code unit would do, but 0xff looks like an obvious sentinel.
            // Can't be 0x00 since that is a valid codepoint.
            *b = 0xff;
        }
    }

    /// Checks whether the ring buffer is empty.
    ///
    /// This function takes O(1) time.
    pub fn is_empty(&self) -> bool {
        let buffer = self.buffer.as_ref();
        self.position == 0 &&
            (buffer.len() == 0 || buffer[buffer.len() - 1] == 0xff)
    }

    fn rotate(&mut self) {
        self.buffer.as_mut().rotate_left(self.position);
        self.position = 0;
    }

    /// Extracts the contents of the ring buffer as a string slice, excluding any
    /// partially overwritten UTF-8 code unit sequences at the beginning.
    ///
    /// Extraction rotates the contents of the ring buffer such that all of its
    /// contents becomes contiguous in memory.
    ///
    /// This function takes O(n) time where n is buffer length.
    pub fn extract(&mut self) -> &str {
        self.rotate();

        // Skip any non-leading UTF-8 code units at the start.
        fn is_utf8_leader(byte: u8) -> bool {
            byte & 0b10000000 == 0b00000000 ||
            byte & 0b11100000 == 0b11000000 ||
            byte & 0b11110000 == 0b11100000 ||
            byte & 0b11111000 == 0b11110000
        }

        let buffer = self.buffer.as_mut();
        for i in 0..buffer.len() {
            if is_utf8_leader(buffer[i]) {
                return core::str::from_utf8(&buffer[i..]).unwrap()
            }
        }
        return ""
    }

    /// Extracts the contents of the ring buffer as an iterator over its lines,
    /// excluding any partially overwritten lines at the beginning.
    ///
    /// The first line written to the ring buffer after clearing it should start
    /// with `'\n'`, or it will be treated as partially overwritten and lost.
    ///
    /// Extraction rotates the contents of the ring buffer such that all of its
    /// contents becomes contiguous in memory.
    ///
    /// This function takes O(n) time where n is buffer length.
    pub fn extract_lines(&mut self) -> core::str::Lines {
        self.rotate();

        let buffer = self.buffer.as_mut();
        for i in 0..buffer.len() {
            if i > 0 && buffer[i - 1] == b'\n' {
                let slice = core::str::from_utf8(&buffer[i..]).unwrap();
                return slice.lines()
            }
        }
        return "".lines()
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> core::fmt::Write for LogBuffer<T> {
    /// Append `s` to the ring buffer.
    ///
    /// This function takes O(n) time where n is length of `s`.
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &b in s.as_bytes() {
            self.buffer.as_mut()[self.position] = b;
            self.position = (self.position + 1) % self.buffer.as_mut().len()
        }
        Ok(())
    }
}
