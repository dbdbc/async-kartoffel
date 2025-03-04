#![no_std]

use async_kartoffel::Position;
use pos::GpsAnchor;

pub mod beacon;
pub mod const_graph;
pub mod gps;
pub mod map;
pub mod pos;

pub type GlobalPos = Position<GpsAnchor>;
