//! log_buffer provides a way to record and extract logs without allocation.
//! It achieves this by utilizing a ring buffer, similar to a *nix _dmesg_ facility.
//!
//! # Usage example
//!
//! ```
//! use log_buffer::LogBuffer;
//! use std::fmt::Write;
//!
//! let mut log_storage = [0; 16];
//! let mut log_buffer  = LogBuffer::new(&mut log_storage);
//! write!(log_buffer, "\nfirst\n").unwrap();
//! write!(log_buffer, "second\n").unwrap();
//! write!(log_buffer, "third\n").unwrap();
//!
//! assert_eq!(log_buffer.extract(),
//!            "st\nsecond\nthird\n");
//! assert_eq!(log_buffer.extract_lines().collect::<Vec<_>>(),
//!            vec!["second", "third"]);
//! ```

#![no_std]

/// A ring buffer that stores UTF-8 text.
#[derive(Debug)]
pub struct LogBuffer<'a> {
    buffer:   &'a mut [u8],
    position: usize
}

impl<'a> LogBuffer<'a> {
    /// Creates a new ring buffer, backed by the slice `storage`.
    ///
    /// The buffer is cleared after creation.
    pub fn new(storage: &'a mut [u8]) -> LogBuffer<'a> {
        let mut buffer = LogBuffer { buffer: { storage }, position: 0 };
        buffer.clear();
        buffer
    }

    /// Clears the buffer.
    ///
    /// Only the text written after clearing will be read out by a future extraction.
    ///
    /// This function takes O(n) time where n is buffer length.
    pub fn clear(&mut self) {
        self.position = 0;
        for b in self.buffer.iter_mut() {
            // Any non-leading UTF-8 code unit would do, but 0xff looks like an obvious sentinel.
            // Can't be 0x00 since that is a valid codepoint.
            *b = 0xff;
        }
    }

    fn rotate(&mut self) {
        // We're rearranging the buffer such that the last written byte is at the last possible
        // index; then we skip all the junk at the start, and only valid UTF-8 should remain.
        let rotate_by = self.position;
        self.position = 0;

        // The Juggling algorithm
        fn gcd(mut a: usize, mut b: usize) -> usize {
            if a < b { core::mem::swap(&mut a, &mut b) }

            while b != 0 {
                let r = a % b;
                a = b;
                b = r;
            }
            a
        }

        for i in 0..gcd(self.buffer.len(), rotate_by) {
          // move i-th values of blocks
          let temp = self.buffer[i];
          let mut j = i;
          loop {
            let mut k = j + rotate_by;
            if k >= self.buffer.len() {
                k = k - self.buffer.len()
            }
            if k == i {
                break
            }
            self.buffer[j] = self.buffer[k];
            j = k;
          }
          self.buffer[j] = temp
        }
    }

    /// Extracts the contents of the ring buffer as a string slice, excluding any
    /// partially overwritten UTF-8 codepoints at the beginning.
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

        for i in 0..self.buffer.len() {
            if is_utf8_leader(self.buffer[i]) {
                return core::str::from_utf8(&self.buffer[i..]).unwrap()
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
    pub fn extract_lines<'b>(&'b mut self) -> core::str::Lines<'b> {
        self.rotate();

        for i in 0..self.buffer.len() {
            if i > 0 && self.buffer[i - 1] == 0x0a {
                let slice = core::str::from_utf8(&self.buffer[i..]).unwrap();
                return slice.lines()
            }
        }
        return "".lines()
    }
}

impl<'a> core::fmt::Write for LogBuffer<'a> {
    /// Append `s` to the ring buffer.
    ///
    /// This function takes O(n) time where n is length of `s`.
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &b in s.as_bytes() {
            self.buffer[self.position] = b;
            self.position = (self.position + 1) % self.buffer.len()
        }
        Ok(())
    }
}
