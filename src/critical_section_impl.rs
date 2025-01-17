// Those functions have to be provided for the critical_section crate. Because the Kartoffel only
// has one thread, they don't need to do anything.

#[no_mangle]
fn _critical_section_1_0_release() {}
#[no_mangle]
fn _critical_section_1_0_acquire() {}
