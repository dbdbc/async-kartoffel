#![no_std]
#![no_main]
#![cfg_attr(feature = "test-kartoffel", feature(custom_test_frameworks))]
#![cfg_attr(feature = "test-kartoffel", reexport_test_harness_main = "test_main")]
#![cfg_attr(feature = "test-kartoffel", test_runner(test_kartoffel::runner))]

use async_kartoffel::Position;
use pos::GpsAnchor;

pub mod beacon;
pub mod const_graph;
pub mod gps;
pub mod map;
pub mod pos;

pub type GlobalPos = Position<GpsAnchor>;

#[cfg(all(test, feature = "test-kartoffel"))]
#[no_mangle]
fn main() {
    test_main();
    loop {}
}
