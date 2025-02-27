#![no_std]

use async_kartoffel::{Position, PositionAnchor};

pub mod beacon;
pub mod const_graph;
pub mod gps;
pub mod map;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpsAnchor {}
impl PositionAnchor for GpsAnchor {}

pub type GlobalPos = Position<GpsAnchor>;
