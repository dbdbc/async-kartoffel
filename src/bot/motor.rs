use crate::{
    mem::{motor_is_ready, motor_step, motor_turn_left, motor_turn_right},
    Error,
};

use core::{future::poll_fn, task::Poll};

use super::Singleton;

#[non_exhaustive]
pub struct Motor;

pub(super) static mut MOTOR: Singleton<Motor> = Singleton {
    instance: Some(Motor),
};

impl Motor {
    pub fn is_ready(&self) -> bool {
        motor_is_ready()
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
            motor_step();
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn step(&mut self) {
        self.wait().await;
        motor_step();
    }

    pub fn try_turn_left(&mut self) -> Result<(), Error> {
        if self.is_ready() {
            motor_turn_left();
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn turn_left(&mut self) {
        self.wait().await;
        motor_turn_left();
    }

    pub fn try_turn_right(&mut self) -> Result<(), Error> {
        if self.is_ready() {
            motor_turn_right();
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn turn_right(&mut self) {
        self.wait().await;
        motor_turn_right();
    }
}
