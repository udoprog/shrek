#![feature(async_await, futures_api, pin, arbitrary_self_types)]

mod resources;
mod dispatcher;
mod system;

pub use crate::{
    resources::{Read, Write, Resources},
    dispatcher::Dispatcher,
    system::System,
};

use std::future::Future;
use std::pin;
use std::sync::mpsc;
use std::sync::{atomic, Arc};
use std::task;

#[derive(Clone)]
pub struct FrameSync {
    pub current: Arc<atomic::AtomicUsize>,
    pub sender: mpsc::Sender<task::Waker>,
}

impl FrameSync {
    /// Synchronize until the next frame.
    pub fn sync(&self) -> FramePoller {
        FramePoller {
            installed: false,
            frame: self.current.load(atomic::Ordering::Acquire),
            current: Arc::clone(&self.current),
            sender: self.sender.clone(),
        }
    }
}

pub struct FramePoller {
    installed: bool,
    frame: usize,
    current: Arc<atomic::AtomicUsize>,
    sender: mpsc::Sender<task::Waker>,
}

impl Future for FramePoller {
    type Output = ();

    fn poll(mut self: pin::Pin<&mut Self>, lw: &task::LocalWaker) -> task::Poll<Self::Output> {
        // NB: this is an overly simplified version that assumes the next time that the future is
        // being polled it is ready.
        if !self.installed {
            self.sender.send(lw.clone().into_waker()).expect("to send");
            self.installed = true;
        }

        if self.frame < self.current.load(atomic::Ordering::Acquire) {
            return task::Poll::Ready(());
        } else {
            return task::Poll::Pending;
        }
    }
}
