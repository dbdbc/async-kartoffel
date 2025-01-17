use core::fmt::Display;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::{
    direction::Direction,
    distance::{Distance, Global},
};

impl Add<Position> for Distance<Global> {
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

impl AddAssign<Distance<Global>> for Position {
    fn add_assign(&mut self, rhs: Distance<Global>) {
        *self = *self + rhs;
    }
}

impl SubAssign<Distance<Global>> for Position {
    fn sub_assign(&mut self, rhs: Distance<Global>) {
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
            (self + Distance::new_east(1), Direction::East),
            (self + Distance::new_west(1), Direction::West),
            (self + Distance::new_north(1), Direction::North),
            (self + Distance::new_south(1), Direction::South),
        ]
    }
    pub const fn to_from_origin(self) -> Distance<Global> {
        Distance::new_global(self.east, self.north)
    }
    pub const fn from_from_origin(dist: Distance<Global>) -> Self {
        Self {
            east: dist.east(),
            north: dist.north(),
        }
    }
}

impl Add<Distance<Global>> for Position {
    type Output = Position;
    fn add(self, rhs: Distance<Global>) -> Self::Output {
        Self {
            east: self.east + rhs.east(),
            north: self.north + rhs.north(),
        }
    }
}

impl Sub<Distance<Global>> for Position {
    type Output = Position;
    fn sub(self, rhs: Distance<Global>) -> Self::Output {
        Self {
            east: self.east - rhs.east(),
            north: self.north - rhs.north(),
        }
    }
}

impl Sub for Position {
    type Output = Distance<Global>;
    fn sub(self, rhs: Self) -> Self::Output {
        Distance::new_global(self.east - rhs.east, self.north - rhs.north)
    }
}
