#![no_std]

extern crate alloc;

use async_kartoffel_generic::Position;
use pos::GpsAnchor;

pub mod beacon;
pub mod const_graph;
pub mod gps;
pub mod map;
pub mod pos;

pub type GlobalPos = Position<GpsAnchor>;
