use core::ptr;

const MEM: *mut u32 = 0x08000000 as *mut u32;
const MEM_TIMER: *mut u32 = MEM;
#[allow(unused)]
const MEM_BATTERY: *mut u32 = MEM.wrapping_byte_add(1024);
const MEM_SERIAL: *mut u32 = MEM.wrapping_byte_add(2 * 1024);
const MEM_MOTOR: *mut u32 = MEM.wrapping_byte_add(3 * 1024);
const MEM_ARM: *mut u32 = MEM.wrapping_byte_add(4 * 1024);
const MEM_RADAR: *mut u32 = MEM.wrapping_byte_add(5 * 1024);
const MEM_COMPASS: *mut u32 = MEM.wrapping_byte_add(6 * 1024);

#[inline(always)]
pub(crate) fn rdi(ptr: *mut u32, off: usize) -> u32 {
    unsafe { ptr::read_volatile(ptr.wrapping_add(off)) }
}

#[inline(always)]
pub(crate) fn wri(ptr: *mut u32, off: usize, val: u32) {
    unsafe {
        ptr::write_volatile(ptr.wrapping_add(off), val);
    }
}

#[inline(always)]
pub(crate) fn arm_is_ready() -> bool {
    rdi(MEM_ARM, 0) == 1
}

#[inline(always)]
pub(crate) fn arm_stab() {
    wri(MEM_ARM, 0, 1);
}

#[inline(always)]
pub(crate) fn arm_pick() {
    wri(MEM_ARM, 0, 2);
}

#[inline(always)]
pub(crate) fn arm_drop(idx: u8) {
    wri(MEM_ARM, 0, u32::from_be_bytes([0, 0, idx, 3]));
}

#[inline(always)]
pub(crate) fn compass_dir() -> u32 {
    rdi(MEM_COMPASS, 0)
}

#[inline(always)]
pub(crate) fn motor_is_ready() -> bool {
    rdi(MEM_MOTOR, 0) == 1
}

#[inline(always)]
pub(crate) fn motor_step() {
    wri(MEM_MOTOR, 0, 1);
}

#[inline(always)]
pub(crate) fn motor_turn_right() {
    wri(MEM_MOTOR, 1, 1);
}

#[inline(always)]
pub(crate) fn motor_turn_left() {
    wri(MEM_MOTOR, 1, u32::MAX);
}

#[inline(always)]
pub(crate) fn radar_is_ready() -> bool {
    rdi(MEM_RADAR, 0) == 1
}

#[inline(always)]
pub(crate) fn radar_scan(size: u32) {
    wri(MEM_RADAR, 0, size);
}

#[inline(always)]
pub(crate) fn radar_get_ex(r: u8, dx: i8, dy: i8, z: u8) -> u32 {
    let d = (2 * r + 1) as usize;
    let x = (dx + r as i8) as usize;
    let y = (dy + r as i8) as usize;
    let z = z as usize;

    rdi(MEM_RADAR, 1 + z * d * d + y * d + x)
}

#[inline(always)]
pub(crate) fn serial_write(word: u32) {
    wri(MEM_SERIAL, 0, word);
}

#[inline(always)]
pub(crate) fn timer_seed() -> u32 {
    rdi(MEM_TIMER, 0)
}

#[inline(always)]
pub(crate) fn timer_ticks() -> u32 {
    rdi(MEM_TIMER, 1)
}
