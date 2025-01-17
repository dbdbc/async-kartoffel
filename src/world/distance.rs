use core::{
    marker::PhantomData,
    ops::{Add, Mul, Neg, Sub},
};

use private::Sealed;

use super::{Direction, Rotation};

mod private {
    pub trait Sealed {}
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum Global {}
impl Sealed for Global {}
impl Coords for Global {}

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum Local {}
impl Sealed for Local {}
impl Coords for Local {}

pub trait Coords: Sealed + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + 'static {}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Distance<C: Coords> {
    data: [i16; 2],
    phantom: PhantomData<C>,
}
impl<C: Coords> Default for Distance<C> {
    fn default() -> Self {
        Self {
            data: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl core::fmt::Debug for Distance<Global> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(East: {}, North: {})", self.data[0], self.data[1])
    }
}

impl core::fmt::Debug for Distance<Local> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(Right: {}, Front: {})", self.data[0], self.data[1])
    }
}

impl Distance<Global> {
    pub const fn local(self, direction: Direction) -> Distance<Local> {
        Distance::<Local> {
            data: match direction {
                Direction::North => [self.data[0], self.data[1]],
                Direction::South => [-self.data[0], -self.data[1]],
                Direction::East => [-self.data[1], self.data[0]],
                Direction::West => [self.data[1], -self.data[0]],
            },
            phantom: PhantomData,
        }
    }
    pub const fn new_east(dist: i16) -> Self {
        Self::new_global(dist, 0)
    }
    pub const fn new_west(dist: i16) -> Self {
        Self::new_global(-dist, 0)
    }
    pub const fn new_north(dist: i16) -> Self {
        Self::new_global(0, dist)
    }
    pub const fn new_south(dist: i16) -> Self {
        Self::new_global(0, -dist)
    }
    pub const fn new_global(east: i16, north: i16) -> Self {
        Self {
            data: [east, north],
            phantom: PhantomData,
        }
    }
    pub const fn east(&self) -> i16 {
        self.data[0]
    }
    pub const fn north(&self) -> i16 {
        self.data[1]
    }
    pub const fn west(&self) -> i16 {
        -self.data[0]
    }
    pub const fn south(&self) -> i16 {
        -self.data[1]
    }
    pub const fn from_direction(direction: Direction, distance: i16) -> Self {
        Distance::<Local>::new_front(distance).global(direction)
    }
    pub const fn get(&self, direction: Direction) -> i16 {
        match direction {
            Direction::North => self.north(),
            Direction::South => self.south(),
            Direction::East => self.east(),
            Direction::West => self.west(),
        }
    }
}
impl Distance<Local> {
    pub const fn new_local(right: i16, front: i16) -> Self {
        Self {
            data: [right, front],
            phantom: PhantomData,
        }
    }
    pub const fn new_front(distance: i16) -> Self {
        Self::new_local(0, distance)
    }
    pub const fn new_back(distance: i16) -> Self {
        Self::new_local(0, -distance)
    }
    pub const fn new_right(distance: i16) -> Self {
        Self::new_local(distance, 0)
    }
    pub const fn new_left(distance: i16) -> Self {
        Self::new_local(-distance, 0)
    }
    pub const fn from_rotation(rotation: Rotation, distance: i16) -> Self {
        Distance::<Local> {
            data: match rotation {
                Rotation::Id => [0, distance],
                Rotation::Left => [-distance, 0],
                Rotation::Right => [distance, 0],
                Rotation::Inverse => [0, -distance],
            },
            phantom: PhantomData,
        }
    }
    pub const fn global(self, direction: Direction) -> Distance<Global> {
        Distance::<Global> {
            data: match direction {
                Direction::North => [self.data[0], self.data[1]],
                Direction::South => [-self.data[0], -self.data[1]],
                Direction::East => [self.data[1], -self.data[0]],
                Direction::West => [-self.data[1], self.data[0]],
            },
            phantom: PhantomData,
        }
    }
    pub const fn front(self) -> i16 {
        self.data[1]
    }
    pub const fn right(self) -> i16 {
        self.data[0]
    }
    pub const fn left(self) -> i16 {
        -self.data[0]
    }
    pub const fn back(self) -> i16 {
        -self.data[1]
    }
}

impl<C: Coords> Distance<C> {
    pub const fn new_generic(i1: i16, i2: i16) -> Self {
        Self {
            data: [i1, i2],
            phantom: PhantomData,
        }
    }
    pub const fn to_generic(self) -> (i16, i16) {
        (self.data[0], self.data[1])
    }
    pub const fn rotate(self, rotator: Rotation) -> Self {
        Self {
            data: match rotator {
                Rotation::Id => [self.data[0], self.data[1]],
                Rotation::Inverse => [-self.data[0], -self.data[1]],
                Rotation::Right => [self.data[1], -self.data[0]],
                Rotation::Left => [-self.data[1], self.data[0]],
            },
            phantom: PhantomData,
        }
    }
}

impl<C: Coords> Sub for Distance<C> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            data: [self.data[0] - rhs.data[0], self.data[1] - rhs.data[1]],
            phantom: PhantomData,
        }
    }
}

impl<C: Coords> Add for Distance<C> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            data: [self.data[0] + rhs.data[0], self.data[1] + rhs.data[1]],
            phantom: PhantomData,
        }
    }
}

impl<C: Coords> Neg for Distance<C> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            data: [-self.data[0], -self.data[1]],
            phantom: PhantomData,
        }
    }
}

impl<C: Coords> Mul<i16> for Distance<C> {
    type Output = Self;

    fn mul(self, rhs: i16) -> Self::Output {
        Self {
            data: [self.data[0] * rhs, self.data[1] * rhs],
            phantom: PhantomData,
        }
    }
}
