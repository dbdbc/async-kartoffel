use core::{future::poll_fn, task::Poll};

use crate::{rdi, wri, Error, Singleton, MEM_ARM};

#[non_exhaustive]
pub struct Arm;

impl Arm {
    #[inline(always)]
    pub fn is_ready(&self) -> bool {
        rdi(MEM_ARM, 0) == 1
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

    pub fn try_stab(&mut self) -> Result<(), Error> {
        if self.is_ready() {
            wri(MEM_ARM, 0, 1);
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn stab(&mut self) {
        self.wait().await;
        wri(MEM_ARM, 0, 1);
    }

    pub fn try_pick(&mut self) -> Result<(), Error> {
        if self.is_ready() {
            wri(MEM_ARM, 0, 2);
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn pick(&mut self) {
        self.wait().await;
        wri(MEM_ARM, 0, 2);
    }

    // TODO
    pub fn try_drop(&mut self, idx: u8) -> Result<(), Error> {
        if self.is_ready() {
            wri(MEM_ARM, 0, u32::from_be_bytes([0, 0, idx, 3]));
            Ok(())
        } else {
            Err(Error::NotReady)
        }
    }
    pub async fn drop(&mut self, idx: u8) {
        self.wait().await;
        wri(MEM_ARM, 0, u32::from_be_bytes([0, 0, idx, 3]));
    }
}

pub static mut ARM: Singleton<Arm> = Singleton {
    instance: Some(Arm),
};
