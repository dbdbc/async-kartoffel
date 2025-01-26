use core::{future::poll_fn, task::Poll};

use kartoffel::compass_dir;

use crate::{Direction, Error};

use super::Singleton;

#[non_exhaustive]
pub struct Compass;

pub(super) static mut COMPASS: Singleton<Compass> = Singleton {
    instance: Some(Compass),
};

impl Compass {
    pub async fn direction(&mut self) -> Direction {
        poll_fn(|cx| match compass_dir() {
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
    pub fn try_direction(&mut self) -> Result<Direction, Error> {
        let result = compass_dir();
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
