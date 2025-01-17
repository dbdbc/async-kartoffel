//! API for creating your own bot in [kartoffels](https://kartoffels.pwy.io) -
//! see the in-game tutorial to get started!

#![no_std]

extern crate alloc;

mod allocator;
mod bot;
mod clock;
mod critical_section_impl;
mod mem;
mod panic;
mod serial;
mod world;

pub use self::bot::{
    Arm, Bot, Compass, Motor, Radar, RadarScan, RadarScanWeak, RadarSize, D3, D5, D7, D9,
};
pub use self::clock::{Cooldown, CooldownType, Duration, Instant, Timer};
pub use self::serial::{SerialControlCode, SerialOutput};
pub use self::world::{Coords, Direction, Distance, Global, Local, Position, Rotation, Tile};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Error {
    NotReady,
    Blocked,
    OutOfMemory,
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => {{
        use ::core::fmt::Write;
        write!(::async_kartoffel::SerialOutput, $($t)*).unwrap();
    }};
}

#[macro_export]
macro_rules! println {
    ($($t:tt)*) => {{
        use ::core::fmt::Write;
        writeln!(::async_kartoffel::SerialOutput, $($t)*).unwrap();
    }};
}

/// Returns a pseudorandom number that can be used as a source of randomness
/// for hashmaps and the like.
///
/// Note that this doesn't return a *new* random number each time it's called -
/// rather the number is randomized once, when the bot is being (re)started.
#[inline(always)]
pub fn random_seed() -> u32 {
    self::mem::timer_seed()
}

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .global _start
    .section .init, "ax"

    _start:
        la sp, _stack_end
        jal main
        ebreak
    "#,
);
