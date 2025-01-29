#![no_std]
#![no_main]
// for tests
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(tests::runner)]

extern crate alloc;

pub mod algorithm;
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
pub use self::world::{Coords, Direction, Distance, Global, Local, Position, Rotation, Tile};
pub use kartoffel::{print, println, serial_buffer, serial_clear, serial_flush, serial_write};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Error {
    NotReady,
    Blocked,
    OutOfMemory,
    Inconsistent,
    NoTargetError,
}

#[inline(always)]
pub fn random_seed() -> u32 {
    timer_seed()
}

#[cfg(test)]
#[no_mangle]
fn main() {
    test_main();
    loop {}
}

#[cfg(test)]
mod tests {

    macro_rules! assert_err {
        ($t1:expr, $t2: expr) => {{
            match $t1 {
                Err(err) => {
                    if err != $t2 {
                        return Err(TestError);
                    }
                }
                _ => return Err(TestError),
            }
        }};
    }
    macro_rules! assert_none {
        ($t:expr) => {{
            match $t {
                None => (),
                Some(_) => return Err(TestError),
            }
        }};
    }
    macro_rules! assert_eq {
        ($t1:expr, $t2:expr) => {{
            if $t1 != $t2 {
                return Err(TestError);
            }
        }};
    }
    macro_rules! assert {
        ($t:expr) => {{
            if !$t {
                return Err(TestError);
            }
        }};
    }
    macro_rules! result_unwrap {
        ($t:expr) => {{
            if let Ok(val) = $t {
                val
            } else {
                return Err(TestError);
            }
        }};
    }
    macro_rules! option_unwrap {
        ($t:expr) => {{
            if let Some(val) = $t {
                val
            } else {
                return Err(TestError);
            }
        }};
    }

    pub(crate) use assert;
    pub(crate) use assert_eq;
    pub(crate) use assert_err;
    pub(crate) use assert_none;
    use kartoffel::println;
    pub(crate) use option_unwrap;
    pub(crate) use result_unwrap;

    pub struct TestError;

    pub trait MyTest {
        fn passed(&self) -> bool;
    }

    impl<T, E> MyTest for T
    where
        T: Fn() -> Result<(), E>,
    {
        fn passed(&self) -> bool {
            self().is_ok()
        }
    }

    pub fn runner(tests: &[&dyn MyTest]) {
        println!("Running tests:");
        let mut n_passed = 0;
        let mut n_failed = 0;
        let n_tests = tests.len();
        for (i, t) in tests.iter().enumerate() {
            println!("  -- {} of {}", i + 1, n_tests);
            if t.passed() {
                println!("  -- PASSED");
                n_passed += 1;
            } else {
                println!("  -- FAILED");
                n_failed += 1;
            }
        }
        match n_failed {
            0 => println!("PASSED all {} tests!", n_passed),
            _ => println!("FAILED, passed {}, failed {}.", n_passed, n_failed),
        }
    }
}
