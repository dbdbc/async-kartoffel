use async_kartoffel::{PositionAnchor, Vec2};

use crate::GlobalPos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpsAnchor {}
impl PositionAnchor for GpsAnchor {}

pub const fn pos_east_north(east: i16, north: i16) -> GlobalPos {
    GlobalPos::add_to_anchor(Vec2::new_global(east, north))
}
