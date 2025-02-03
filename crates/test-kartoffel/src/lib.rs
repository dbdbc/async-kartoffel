#![no_std]
#![no_main]
// for tests
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(runner)]

use kartoffel::println;

extern crate alloc;

#[cfg(test)]
#[no_mangle]
fn main() {
    test_main();
    loop {}
}

#[test_case]
fn example_test() -> Result<(), TestError> {
    assert_eq!(1 + 2, 3);
    Ok(())
}

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

#[macro_export]
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
#[macro_export]
macro_rules! assert_none {
    ($t:expr) => {{
        match $t {
            None => (),
            Some(_) => return Err(TestError),
        }
    }};
}
#[macro_export]
macro_rules! assert_eq {
    ($t1:expr, $t2:expr) => {{
        if $t1 != $t2 {
            return Err(TestError);
        }
    }};
}
#[macro_export]
macro_rules! assert {
    ($t:expr) => {{
        if !$t {
            return Err(TestError);
        }
    }};
}
#[macro_export]
macro_rules! result_unwrap {
    ($t:expr) => {{
        if let Ok(val) = $t {
            val
        } else {
            return Err(TestError);
        }
    }};
}
#[macro_export]
macro_rules! option_unwrap {
    ($t:expr) => {{
        if let Some(val) = $t {
            val
        } else {
            return Err(TestError);
        }
    }};
}
