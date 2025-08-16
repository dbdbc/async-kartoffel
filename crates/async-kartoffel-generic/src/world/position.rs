use core::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    ops::{Add, AddAssign, Sub, SubAssign},
};

pub trait PositionAnchor: Clone + Copy + Debug + PartialEq + Eq + PartialOrd + Ord + Hash {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnchorDefault {}
impl PositionAnchor for AnchorDefault {}

use super::{
    direction::Direction,
    vec2::{Global, Vec2},
};

/// anchor is used to differentiate between different definitions of the (0, 0) position
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct Position<Anchor: PositionAnchor = AnchorDefault> {
    east: i16,
    south: i16,
    _phantom: PhantomData<Anchor>,
}

impl<Anchor: PositionAnchor> Default for Position<Anchor> {
    fn default() -> Self {
        Self {
            east: 0,
            south: 0,
            _phantom: PhantomData,
        }
    }
}

impl<Anchor: PositionAnchor> Position<Anchor> {
    pub fn neighbors(self) -> [(Self, Direction); 4] {
        [
            (self + Vec2::new_east(1), Direction::East),
            (self + Vec2::new_west(1), Direction::West),
            (self + Vec2::new_north(1), Direction::North),
            (self + Vec2::new_south(1), Direction::South),
        ]
    }

    /// vector from anchor, anchor is the (0, 0) position
    /// defined because of const
    pub const fn subtract_anchor(self) -> Vec2<Global> {
        Vec2::new_east_south(self.east, self.south)
    }

    /// vector from anchor, anchor is the (0, 0) position
    /// defined because of const
    pub const fn add_to_anchor(vec: Vec2<Global>) -> Self {
        Self {
            east: vec.east(),
            south: vec.south(),
            _phantom: PhantomData,
        }
    }
}

impl<Anchor: PositionAnchor> Display for Position<Anchor> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.east, self.south)
    }
}

impl<Anchor: PositionAnchor> Add<Vec2<Global>> for Position<Anchor> {
    type Output = Self;
    fn add(self, rhs: Vec2<Global>) -> Self::Output {
        Self {
            east: self.east + rhs.east(),
            south: self.south + rhs.south(),
            _phantom: PhantomData,
        }
    }
}

impl<Anchor: PositionAnchor> Sub<Vec2<Global>> for Position<Anchor> {
    type Output = Self;
    fn sub(self, rhs: Vec2<Global>) -> Self::Output {
        Self {
            east: self.east - rhs.east(),
            south: self.south - rhs.south(),
            _phantom: PhantomData,
        }
    }
}

impl<Anchor: PositionAnchor> Sub for Position<Anchor> {
    type Output = Vec2<Global>;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec2::new_east_south(self.east - rhs.east, self.south - rhs.south)
    }
}

impl<Anchor: PositionAnchor> Add<Position<Anchor>> for Vec2<Global> {
    type Output = Position<Anchor>;
    fn add(self, rhs: Position<Anchor>) -> Self::Output {
        Self::Output {
            east: self.east() + rhs.east,
            south: self.south() + rhs.south,
            _phantom: PhantomData,
        }
    }
}

impl<Anchor: PositionAnchor> AddAssign<Vec2<Global>> for Position<Anchor> {
    fn add_assign(&mut self, rhs: Vec2<Global>) {
        *self = *self + rhs;
    }
}

impl<Anchor: PositionAnchor> SubAssign<Vec2<Global>> for Position<Anchor> {
    fn sub_assign(&mut self, rhs: Vec2<Global>) {
        *self = *self - rhs;
    }
}
