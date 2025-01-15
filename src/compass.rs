use core::{
    future::poll_fn,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
    task::Poll,
};

use crate::{rdi, Error, Singleton, MEM_COMPASS};

#[non_exhaustive]
pub struct Compass;

pub static mut COMPASS: Singleton<Compass> = Singleton {
    instance: Some(Compass),
};

impl Compass {
    pub async fn dir(&mut self) -> Direction {
        poll_fn(|cx| match rdi(MEM_COMPASS, 0) {
            0 => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            1 => Poll::Ready(Direction::North),
            2 => Poll::Ready(Direction::East),
            3 => Poll::Ready(Direction::South),
            4 => Poll::Ready(Direction::West),
            _ => unreachable!(),
        })
        .await
    }
    pub fn try_dir(&mut self) -> Result<Direction, Error> {
        let result = rdi(MEM_COMPASS, 0);
        match result {
            0 => Err(Error::NotReady),
            1 => Ok(Direction::North),
            2 => Ok(Direction::East),
            3 => Ok(Direction::South),
            4 => Ok(Direction::West),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub enum Rotation {
    #[default]
    Id,
    Left,
    Right,
    Inverse,
}

impl Direction {
    pub fn all() -> [Self; 4] {
        [Self::North, Self::South, Self::East, Self::West]
    }
    fn to_num(self) -> u8 {
        match self {
            Direction::East => 0,
            Direction::South => 1,
            Direction::West => 2,
            Direction::North => 3,
        }
    }
    fn from_num(x: u8) -> Self {
        match x.rem_euclid(4) {
            0 => Direction::East,
            1 => Direction::South,
            2 => Direction::West,
            3 => Direction::North,
            _ => unreachable!(),
        }
    }
}

impl AddAssign<Rotation> for Direction {
    fn add_assign(&mut self, rhs: Rotation) {
        *self = *self + rhs
    }
}

impl SubAssign<Rotation> for Direction {
    fn sub_assign(&mut self, rhs: Rotation) {
        *self = *self - rhs
    }
}

impl Add<Rotation> for Direction {
    type Output = Direction;

    fn add(self, rhs: Rotation) -> Self::Output {
        Self::from_num(self.to_num() + rhs.to_num())
    }
}

impl Sub<Rotation> for Direction {
    type Output = Direction;

    fn sub(self, rhs: Rotation) -> Self::Output {
        self + (-rhs)
    }
}

impl Sub for Direction {
    type Output = Rotation;

    fn sub(self, rhs: Self) -> Self::Output {
        // add 4 to keep positive
        Rotation::from_num(4 + self.to_num() - rhs.to_num())
    }
}

impl AddAssign for Rotation {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Rotation {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Rotation {
    pub fn all() -> [Self; 4] {
        [Self::Id, Self::Left, Self::Right, Self::Inverse]
    }
    fn to_num(self) -> u8 {
        match self {
            Rotation::Id => 0,
            Rotation::Left => 3,
            Rotation::Right => 1,
            Rotation::Inverse => 2,
        }
    }
    fn from_num(x: u8) -> Self {
        match x.rem_euclid(4) {
            0 => Rotation::Id,
            1 => Rotation::Right,
            2 => Rotation::Inverse,
            3 => Rotation::Left,
            _ => unreachable!(),
        }
    }
}

impl Neg for Rotation {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Rotation::Id => Rotation::Id,
            Rotation::Left => Rotation::Right,
            Rotation::Right => Rotation::Left,
            Rotation::Inverse => Rotation::Inverse,
        }
    }
}

impl Add for Rotation {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from_num(self.to_num() + rhs.to_num())
    }
}

impl Add<Direction> for Rotation {
    type Output = Direction;

    fn add(self, rhs: Direction) -> Self::Output {
        rhs + self
    }
}

impl Sub for Rotation {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}
