use async_kartoffel_generic::{PositionAnchor, Vec2};

use crate::GlobalPos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpsAnchor {}
impl PositionAnchor for GpsAnchor {}

pub const fn pos_east_south(east: i16, south: i16) -> GlobalPos {
    GlobalPos::add_to_anchor(Vec2::new_east_south(east, south))
}
