// Those functions have to be provided for the critical_section crate. Because the Kartoffel only
// has one thread, they don't need to do anything.
// https://docs.rs/critical-section/latest/critical_section/#providing-an-implementation

#[unsafe(no_mangle)]
fn _critical_section_1_0_release() {}
#[unsafe(no_mangle)]
fn _critical_section_1_0_acquire() {}
