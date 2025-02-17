use kartoffel::{is_motor_ready, motor_step_bw, motor_step_fw, motor_turn_left, motor_turn_right};

use core::{future::poll_fn, task::Poll};

use super::{error::NotReady, Singleton};

#[non_exhaustive]
pub struct Motor;

pub(super) static mut MOTOR: Singleton<Motor> = Singleton {
    instance: Some(Motor),
};

impl Motor {
    pub fn is_ready(&self) -> bool {
        is_motor_ready()
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

    pub fn try_step_fw(&mut self) -> Result<(), NotReady> {
        if self.is_ready() {
            motor_step_fw();
            Ok(())
        } else {
            Err(NotReady)
        }
    }
    pub async fn step_fw(&mut self) {
        self.wait().await;
        motor_step_fw();
    }

    pub fn try_step_bw(&mut self) -> Result<(), NotReady> {
        if self.is_ready() {
            motor_step_bw();
            Ok(())
        } else {
            Err(NotReady)
        }
    }
    pub async fn step_bw(&mut self) {
        self.wait().await;
        motor_step_bw();
    }

    pub fn try_turn_left(&mut self) -> Result<(), NotReady> {
        if self.is_ready() {
            motor_turn_left();
            Ok(())
        } else {
            Err(NotReady)
        }
    }
    pub async fn turn_left(&mut self) {
        self.wait().await;
        motor_turn_left();
    }

    pub fn try_turn_right(&mut self) -> Result<(), NotReady> {
        if self.is_ready() {
            motor_turn_right();
            Ok(())
        } else {
            Err(NotReady)
        }
    }
    pub async fn turn_right(&mut self) {
        self.wait().await;
        motor_turn_right();
    }
}
