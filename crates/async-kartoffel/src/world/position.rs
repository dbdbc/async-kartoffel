use core::fmt::Display;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::{
    direction::Direction,
    vec2::{Global, Vec2},
};

impl Add<Position> for Vec2<Global> {
    type Output = Position;
    fn add(self, rhs: Position) -> Self::Output {
        Position {
            east: self.east() + rhs.east,
            north: self.north() + rhs.north,
        }
    }
}

#[derive(Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct Position {
    east: i16,
    north: i16,
}

impl AddAssign<Vec2<Global>> for Position {
    fn add_assign(&mut self, rhs: Vec2<Global>) {
        *self = *self + rhs;
    }
}

impl SubAssign<Vec2<Global>> for Position {
    fn sub_assign(&mut self, rhs: Vec2<Global>) {
        *self = *self - rhs;
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.east, self.north)
    }
}

impl Position {
    pub fn neighbors(self) -> [(Self, Direction); 4] {
        [
            (self + Vec2::new_east(1), Direction::East),
            (self + Vec2::new_west(1), Direction::West),
            (self + Vec2::new_north(1), Direction::North),
            (self + Vec2::new_south(1), Direction::South),
        ]
    }
    pub const fn to_from_origin(self) -> Vec2<Global> {
        Vec2::new_global(self.east, self.north)
    }
    pub const fn from_from_origin(vec: Vec2<Global>) -> Self {
        Self {
            east: vec.east(),
            north: vec.north(),
        }
    }
}

impl Add<Vec2<Global>> for Position {
    type Output = Position;
    fn add(self, rhs: Vec2<Global>) -> Self::Output {
        Self {
            east: self.east + rhs.east(),
            north: self.north + rhs.north(),
        }
    }
}

impl Sub<Vec2<Global>> for Position {
    type Output = Position;
    fn sub(self, rhs: Vec2<Global>) -> Self::Output {
        Self {
            east: self.east - rhs.east(),
            north: self.north - rhs.north(),
        }
    }
}

impl Sub for Position {
    type Output = Vec2<Global>;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec2::new_global(self.east - rhs.east, self.north - rhs.north)
    }
}
