#![no_std]
#![no_main]
#![cfg_attr(feature = "test-kartoffel", feature(custom_test_frameworks))]
#![cfg_attr(feature = "test-kartoffel", reexport_test_harness_main = "test_main")]
#![cfg_attr(feature = "test-kartoffel", test_runner(test_kartoffel::runner))]

extern crate alloc;

mod bot;
mod clock;
#[cfg(feature = "critical-section-impl")]
mod critical_section_impl;

use kartoffel::timer_seed;

pub use bot::{Arm, Bot, Compass, Motor, Radar, RadarScan, RadarScanWeak};
pub use clock::{Duration, Instant, KartoffelClock, Timer};

#[cfg(target_arch = "riscv32")]
pub use kartoffel::{print, println};

pub use kartoffel::{serial_buffer, serial_clear, serial_flush, serial_write};

#[inline(always)]
pub fn random_seed() -> u32 {
    timer_seed()
}

#[cfg(all(test, feature = "test-kartoffel"))]
#[unsafe(no_mangle)]
fn main() {
    test_main();
    loop {}
}

/// kill bot
pub fn exit() {
    unsafe {
        core::ptr::read_volatile(0xDEAD_0000 as *const u32);
    }
}
