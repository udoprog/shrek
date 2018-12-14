#![feature(await_macro, async_await, futures_api, pin, arbitrary_self_types)]

use shrek::{FrameSync, Read, Write, System, Resources, Dispatcher};
use std::future::Future;
use std::pin;
use std::sync::mpsc;
use std::task;

#[derive(Clone)]
pub struct Example {
    counter: u32,
}

impl<'a> System<'a> for Example {
    type Data = (
        Read<'a, FrameSync>,
        Write<'a, u32>,
    );

    fn run(mut self, (frame, mut number): Self::Data) -> pin::Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pinned(
            async move {
                // TODO: how do we prevent this?
                let num = &mut *number;

                loop {
                    await!(frame.sync());

                    println!("tick: {}", self.counter);
                    println!("thing: {}", *num);
                    self.counter += 1;
                    *num += 1;
                }
            },
        )
    }
}

fn main() {
    use std::sync::{atomic, Arc};
    use std::thread;
    use std::time;

    let current = Arc::new(atomic::AtomicUsize::default());

    let (tx, rx) = mpsc::channel::<task::Waker>();

    {
        let current = Arc::clone(&current);

        // thread responsible for 'ticking'.
        thread::spawn(move || loop {
            thread::sleep(time::Duration::from_millis(500));

            current.fetch_add(1, atomic::Ordering::Release);

            rx.recv().unwrap().wake();
            rx.recv().unwrap().wake();
        });
    };

    let value = 42;

    let a = Example {
        counter: 0,
    };

    let b = Example {
        counter: 0,
    };

    let mut resources = Resources::new();

    resources.add_resource(FrameSync {
        current,
        sender: tx.clone(),
    });

    resources.add_resource(42u32);

    let mut dispatcher = Dispatcher::new();

    dispatcher.with(&mut resources, a);
    dispatcher.with(&mut resources, b);

    futures::executor::block_on(
        async {
            await!(dispatcher.into_future(resources));
        },
    );
}