use core::{
    cmp::Ordering,
    marker::PhantomData,
    ops::{Add, Mul, Neg, Sub},
};

use private::Sealed;

use super::{Direction, Rotation};

mod private {
    pub trait Sealed {}
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Global {}
impl Sealed for Global {}
impl Coords for Global {}

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Local {}
impl Sealed for Local {}
impl Coords for Local {}

pub trait Coords: Sealed + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + 'static {}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Vec2<C: Coords> {
    data: [i16; 2],
    phantom: PhantomData<C>,
}
impl<C: Coords> Default for Vec2<C> {
    fn default() -> Self {
        Self {
            data: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl core::fmt::Debug for Vec2<Global> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(East: {}, North: {})", self.data[0], self.data[1])
    }
}

impl core::fmt::Debug for Vec2<Local> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(Right: {}, Front: {})", self.data[0], self.data[1])
    }
}

impl Vec2<Global> {
    pub const fn local(self, direction: Direction) -> Vec2<Local> {
        Vec2::<Local> {
            data: match direction {
                Direction::North => [self.data[0], self.data[1]],
                Direction::South => [-self.data[0], -self.data[1]],
                Direction::East => [-self.data[1], self.data[0]],
                Direction::West => [self.data[1], -self.data[0]],
            },
            phantom: PhantomData,
        }
    }
    pub const fn new_east(distance: i16) -> Self {
        Self::new_global(distance, 0)
    }
    pub const fn new_west(distance: i16) -> Self {
        Self::new_global(-distance, 0)
    }
    pub const fn new_north(distance: i16) -> Self {
        Self::new_global(0, distance)
    }
    pub const fn new_south(distance: i16) -> Self {
        Self::new_global(0, -distance)
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
        Vec2::<Local>::new_front(distance).global(direction)
    }
    pub const fn get(&self, direction: Direction) -> i16 {
        match direction {
            Direction::North => self.north(),
            Direction::South => self.south(),
            Direction::East => self.east(),
            Direction::West => self.west(),
        }
    }

    pub fn directions(self) -> &'static [Direction] {
        match (
            self.get(Direction::East).cmp(&0),
            self.get(Direction::North).cmp(&0),
        ) {
            (Ordering::Less, Ordering::Less) => &[Direction::West, Direction::South],
            (Ordering::Less, Ordering::Equal) => &[Direction::West],
            (Ordering::Less, Ordering::Greater) => &[Direction::West, Direction::North],
            (Ordering::Equal, Ordering::Less) => &[Direction::South],
            (Ordering::Equal, Ordering::Equal) => &[],
            (Ordering::Equal, Ordering::Greater) => &[Direction::North],
            (Ordering::Greater, Ordering::Less) => &[Direction::East, Direction::South],
            (Ordering::Greater, Ordering::Equal) => &[Direction::East],
            (Ordering::Greater, Ordering::Greater) => &[Direction::East, Direction::North],
        }
    }
}
impl Vec2<Local> {
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
        Vec2::<Local> {
            data: match rotation {
                Rotation::Id => [0, distance],
                Rotation::Left => [-distance, 0],
                Rotation::Right => [distance, 0],
                Rotation::Inverse => [0, -distance],
            },
            phantom: PhantomData,
        }
    }
    pub const fn global(self, direction: Direction) -> Vec2<Global> {
        Vec2::<Global> {
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

impl<C: Coords> Vec2<C> {
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

impl<C: Coords> Sub for Vec2<C> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            data: [self.data[0] - rhs.data[0], self.data[1] - rhs.data[1]],
            phantom: PhantomData,
        }
    }
}

impl<C: Coords> Add for Vec2<C> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            data: [self.data[0] + rhs.data[0], self.data[1] + rhs.data[1]],
            phantom: PhantomData,
        }
    }
}

impl<C: Coords> Neg for Vec2<C> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            data: [-self.data[0], -self.data[1]],
            phantom: PhantomData,
        }
    }
}

impl<C: Coords> Mul<i16> for Vec2<C> {
    type Output = Self;

    fn mul(self, rhs: i16) -> Self::Output {
        Self {
            data: [self.data[0] * rhs, self.data[1] * rhs],
            phantom: PhantomData,
        }
    }
}
