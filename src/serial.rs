use core::fmt;

use crate::mem::serial_write;

/// Terminal control code - similar to ANSI color code, i.e. it allows to
/// manipulate the terminal.
///
/// # Examples
///
/// ## Reducing flickering
///
/// If you plan on displaying something animated, the terminal might flicker -
/// you can get rid of this like so:
///
/// ```rust,no_run
/// # use kartoffel::*;
/// #
/// let mut n = 0;
///
/// loop {
///     serial_write(SerialControlCode::StartBuffering);
///     serial_write("hello: ");
///     serial_write(format!("{n}"));
///     serial_write(SerialControlCode::FlushBuffer);
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SerialControlCode {
    /// Start buffering the output.
    ///
    /// All characters sent from this point on will not be displayed until you
    /// send [`SerialControlCode::FlushBuffer`].
    StartBuffering,

    /// Flush the buffered output and print it on the terminal.
    FlushBuffer,
}

impl SerialControlCode {
    pub fn encode(&self) -> u32 {
        let ctrl = match self {
            SerialControlCode::StartBuffering => 0x00,
            SerialControlCode::FlushBuffer => 0x01,
        };

        0xffffff00 | ctrl
    }
    pub fn write(&self) {
        serial_write(self.encode());
    }
}

/// A dummy struct for writing formatted strings to the serial port.
///
/// Implements `fmt::Write` trait, so you can use it with `write!` to write
/// formatted strings to the serial port, totally without any allocations.
///
/// # Example
///
/// ```no_run
/// use kartoffel::*;
/// use core::fmt::Write;
///
/// let mut serial = SerialOutput;
/// write!(&mut serial, "Hello, {}!", "world").unwrap();
/// ```
pub struct SerialOutput;

impl fmt::Write for SerialOutput {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            serial_write(c as u32);
        }
        Ok(())
    }
}
