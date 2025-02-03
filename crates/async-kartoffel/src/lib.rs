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
mod world;

use kartoffel::timer_seed;

pub use self::bot::{
    Arm, Bot, Compass, Motor, Radar, RadarScan, RadarScanWeak, RadarSize, D3, D5, D7, D9,
};
pub use self::clock::{Cooldown, CooldownType, Duration, Instant, Timer};
pub use self::world::{Coords, Direction, Global, Local, Position, Rotation, Tile, Vec2};

pub use kartoffel::{print, println, serial_buffer, serial_clear, serial_flush, serial_write};

#[inline(always)]
pub fn random_seed() -> u32 {
    timer_seed()
}

#[cfg(all(test, feature = "test-kartoffel"))]
#[no_mangle]
fn main() {
    test_main();
    loop {}
}
