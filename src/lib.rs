//! API for creating your own bot in [kartoffels](https://kartoffels.pwy.io) -
//! see the in-game tutorial to get started!

#![no_std]

extern crate alloc;

mod allocator;
mod arm;
mod compass;
mod motor;
mod panic;
mod radar;
mod serial;
mod timer;

pub use self::arm::Arm;
pub use self::compass::{Compass, Direction, Rotation};
pub use self::motor::Motor;
pub use self::radar::{Radar, RadarScan, RadarScanWeak, RadarSize, D3, D5, D7, D9};
pub use self::serial::*;
pub use self::timer::*;
use core::ptr;

const MEM: *mut u32 = 0x08000000 as *mut u32;
const MEM_TIMER: *mut u32 = MEM;
const MEM_BATTERY: *mut u32 = MEM.wrapping_byte_add(1024);
const MEM_SERIAL: *mut u32 = MEM.wrapping_byte_add(2 * 1024);
const MEM_MOTOR: *mut u32 = MEM.wrapping_byte_add(3 * 1024);
const MEM_ARM: *mut u32 = MEM.wrapping_byte_add(4 * 1024);
const MEM_RADAR: *mut u32 = MEM.wrapping_byte_add(5 * 1024);
const MEM_COMPASS: *mut u32 = MEM.wrapping_byte_add(6 * 1024);

pub(crate) struct Singleton<T> {
    instance: Option<T>,
}
impl<T> Singleton<T> {
    pub fn take(&mut self) -> T {
        let instance = core::mem::take(&mut self.instance);
        instance.unwrap()
    }
}

pub struct Bot {
    pub motor: Motor,
    pub radar: Radar,
    pub arm: Arm,
    pub compass: Compass,
}

impl Bot {
    pub fn take() -> Self {
        #[allow(static_mut_refs)]
        Bot {
            motor: unsafe { motor::MOTOR.take() },
            arm: unsafe { arm::ARM.take() },
            compass: unsafe { compass::COMPASS.take() },
            radar: unsafe { radar::RADAR.take() },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Error {
    NotReady,
    Blocked,
}

#[inline(always)]
fn rdi(ptr: *mut u32, off: usize) -> u32 {
    unsafe { ptr::read_volatile(ptr.wrapping_add(off)) }
}

#[inline(always)]
fn wri(ptr: *mut u32, off: usize, val: u32) {
    unsafe {
        ptr::write_volatile(ptr.wrapping_add(off), val);
    }
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
