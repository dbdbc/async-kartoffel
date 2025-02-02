use core::{future::poll_fn, task::Poll};
use kartoffel::{arm_drop, arm_pick, arm_stab, is_arm_ready};

use super::{error::NotReady, Singleton};

#[non_exhaustive]
pub struct Arm;

pub(super) static mut ARM: Singleton<Arm> = Singleton {
    instance: Some(Arm),
};

impl Arm {
    #[inline(always)]
    pub fn is_ready(&self) -> bool {
        is_arm_ready()
    }

    #[inline(always)]
    pub fn wait_blocking(&self) {
        while !self.is_ready() {
            //
        }
    }

    pub async fn wait(&self) {
        poll_fn(|cx| {
            if self.is_ready() {
                Poll::Ready(())
            } else {
                // TODO register waker instead of polling in loop?
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
    }

    pub fn try_stab(&mut self) -> Result<(), NotReady> {
        if self.is_ready() {
            arm_stab();
            Ok(())
        } else {
            Err(NotReady)
        }
    }
    pub async fn stab(&mut self) {
        self.wait().await;
        arm_stab()
    }

    pub fn try_pick(&mut self) -> Result<(), NotReady> {
        if self.is_ready() {
            arm_pick();
            Ok(())
        } else {
            Err(NotReady)
        }
    }
    pub async fn pick(&mut self) {
        self.wait().await;
        arm_pick();
    }

    // TODO
    pub fn try_drop(&mut self, idx: u8) -> Result<(), NotReady> {
        if self.is_ready() {
            arm_drop(idx);
            Ok(())
        } else {
            Err(NotReady)
        }
    }
    pub async fn drop(&mut self, idx: u8) {
        self.wait().await;
        arm_drop(idx);
    }
}
