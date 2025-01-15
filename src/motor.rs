use crate::{rdi, wri, Error, MEM_MOTOR};

use core::{future::poll_fn, task::Poll};

use super::Singleton;

#[non_exhaustive]
pub struct Motor;

pub static mut MOTOR: Singleton<Motor> = Singleton {
    instance: Some(Motor),
};

impl Motor {
    pub fn is_ready(&self) -> bool {
        rdi(MEM_MOTOR, 0) == 1
    }

    pub async fn wait(&self) {
        poll_fn(|cx| {
            if self.is_ready() {
                Poll::Ready(())
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
    }

    pub fn try_step(&mut self) -> Result<(), Error> {
        if self.is_ready() {
            wri(MEM_MOTOR, 0, 1);
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn step(&mut self) {
        self.wait().await;
        wri(MEM_MOTOR, 0, 1);
    }

    pub fn try_turn_left(&mut self) -> Result<(), Error> {
        if self.is_ready() {
            wri(MEM_MOTOR, 1, u32::MAX);
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn turn_left(&mut self) {
        self.wait().await;
        wri(MEM_MOTOR, 1, u32::MAX);
    }

    pub fn try_turn_right(&mut self) -> Result<(), Error> {
        if self.is_ready() {
            wri(MEM_MOTOR, 1, 1);
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn turn_right(&mut self) {
        self.wait().await;
        wri(MEM_MOTOR, 1, 1);
    }
}
