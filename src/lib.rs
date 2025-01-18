#![no_std]
#![no_main]
// for tests
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(tests::runner)]

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

#[cfg(test)]
#[no_mangle]
fn main() {
    test_main();
    loop {}
}

#[cfg(test)]
mod tests {

    macro_rules! log {
        ($($t:tt)*) => {{
            use ::core::fmt::Write;
            writeln!(crate::SerialOutput, $($t)*).unwrap();
        }};
    }
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
    pub(crate) use log;
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
        log!("Running tests:");
        let mut n_passed = 0;
        let mut n_failed = 0;
        let n_tests = tests.len();
        for (i, t) in tests.iter().enumerate() {
            log!("  -- {} of {}", i + 1, n_tests);
            if t.passed() {
                log!("  -- PASSED");
                n_passed += 1;
            } else {
                log!("  -- FAILED");
                n_failed += 1;
            }
        }
        match n_failed {
            0 => log!("PASSED all {} tests!", n_passed),
            _ => log!("FAILED, passed {}, failed {}.", n_passed, n_failed),
        }
    }
}
