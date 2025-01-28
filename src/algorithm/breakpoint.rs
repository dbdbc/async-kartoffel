use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// A short break in async functions, that allows other tasks to run.
#[derive(Default)]
pub struct Breakpoint {
    yielded_once: bool,
}
impl Breakpoint {
    pub fn new() -> Self {
        Default::default()
    }
}
impl Unpin for Breakpoint {}
impl Future for Breakpoint {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded_once {
            Poll::Ready(())
        } else {
            self.yielded_once = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
