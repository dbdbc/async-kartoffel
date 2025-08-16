use core::{num::NonZeroU64, ops::RangeInclusive};

use crate::{Coords, Local, Tile, Vec2};

mod private {
    pub trait Sealed {}
}
pub trait RadarSize:
    private::Sealed + Clone + Copy + PartialEq + Eq + PartialOrd + Ord + 'static
{
    const R: u8;
    const D: u8;
    fn range() -> RangeInclusive<i8> {
        -(Self::R as i8)..=Self::R as i8
    }
    fn contains(vec: Vec2<impl Coords>) -> bool {
        let (i1, i2) = vec.to_generic();
        -(Self::R as i16) <= i1
            && -(Self::R as i16) <= i2
            && (Self::R as i16) >= i1
            && (Self::R as i16) >= i2
    }
    fn to_str() -> &'static str;
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D3 {}
impl private::Sealed for D3 {}
impl RadarSize for D3 {
    const R: u8 = 1;
    const D: u8 = 3;
    fn to_str() -> &'static str {
        "D3"
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D5 {}
impl private::Sealed for D5 {}
impl RadarSize for D5 {
    const R: u8 = 2;
    const D: u8 = 5;
    fn to_str() -> &'static str {
        "D5"
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D7 {}
impl private::Sealed for D7 {}
impl RadarSize for D7 {
    const R: u8 = 3;
    const D: u8 = 7;
    fn to_str() -> &'static str {
        "D7"
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D9 {}
impl private::Sealed for D9 {}
impl RadarSize for D9 {
    const R: u8 = 4;
    const D: u8 = 9;
    fn to_str() -> &'static str {
        "D9"
    }
}

pub trait RadarScanTrait<Size: RadarSize> {
    fn contains(&self, vec: Vec2<Local>) -> bool;

    fn at(&self, vec: Vec2<Local>) -> Option<Tile>;

    fn bot_at(&self, vec: Vec2<Local>) -> Option<NonZeroU64>;

    /// Scanned tiles matching tile excluding (0, 0), this is e.g. useful to find only enemy bots
    fn iter_tile(&self, tile: Tile) -> impl Iterator<Item = Vec2<Local>> + use<'_, Size, Self>;

    /// iterate over scanned tiles excluding (0, 0)
    fn iter(&self) -> impl Iterator<Item = (Vec2<Local>, Tile)> + use<'_, Size, Self>;
}
