use crate::{
    resources::{Resources, ResourceSpecification, ReadSpecification, WriteSpecification, ResourceReference},
    system::System,
};
use std::pin;
use std::future::Future;
use std::ptr;
use std::task;

/// All data dependencies that this system has.
pub struct SystemAndData<'dispatcher> {
    reads: Vec<ReadSpecification>,
    writes: Vec<WriteSpecification>,
    future: pin::Pin<Box<Future<Output = ()> + 'dispatcher>>,
}

/// A dispatcher is responsible for driving systems.
pub struct Dispatcher<'dispatcher> {
    resource_reference: ptr::NonNull<ResourceReference>,
    systems: Vec<SystemAndData<'dispatcher>>,
}

impl<'dispatcher> Dispatcher<'dispatcher> {
    pub fn new() -> Dispatcher<'dispatcher> {
        Dispatcher {
            resource_reference: unsafe { ptr::NonNull::new_unchecked(Box::into_raw(ResourceReference::new())) },
            systems: Vec::new(),
        }
    }

    /// Add a system to the dispatcher.
    pub fn with<S>(&mut self, resources: &mut Resources, system: S) where S: 'static + System<'dispatcher> {
        unsafe {
            self.resource_reference.as_mut().set_resources(ptr::NonNull::new_unchecked(resources as *mut _));
        }

        let mut reads = Vec::new();
        S::Data::reads(&mut reads);

        let mut writes = Vec::new();
        S::Data::writes(&mut writes);

        let data = S::Data::fetch(self.resource_reference.clone());

        self.systems.push(SystemAndData {
            reads, writes,
            future: system.run(data),
        });

        unsafe {
            self.resource_reference.as_mut().clear_resources();
        }
    }

    /// Just a convenience function to run this dispatcher as a future right now.
    pub fn into_future(self, resources: Resources) -> DispatcherFuture<'dispatcher> {
        DispatcherFuture {
            resources,
            resource_reference: self.resource_reference,
            systems: self.systems,
        }
    }

    /// Run a single iteration of the dispatcher.
    pub fn run(&mut self, resources: &mut Resources) {
        unsafe {
            self.resource_reference.as_mut().set_resources(ptr::NonNull::new_unchecked(resources as *mut _));
        }

        /* do something :/ */

        unsafe {
            self.resource_reference.as_mut().clear_resources();
        }
    }
}

/// Note: this is a temporary implementation of the dispatcher as a future.
pub struct DispatcherFuture<'dispatcher> {
    resource_reference: ptr::NonNull<ResourceReference>,
    systems: Vec<SystemAndData<'dispatcher>>,
    resources: Resources,
}

impl<'dispatcher> Future for DispatcherFuture<'dispatcher> {
    type Output = ();

    fn poll(mut self: pin::Pin<&mut Self>, lw: &task::LocalWaker) -> task::Poll<Self::Output> {
        let DispatcherFuture {
            ref mut resource_reference,
            ref mut systems,
            ref mut resources,
        } = *self;

        unsafe {
            resource_reference.as_mut().set_resources(ptr::NonNull::new_unchecked(resources as *mut _));
        }

        let mut all_done = true;

        for s in systems {
            let fut = unsafe { pin::Pin::new_unchecked(&mut s.future) };

            if let task::Poll::Pending = fut.poll(lw) {
                all_done = false;
            }
        }

        unsafe {
            resource_reference.as_mut().clear_resources();
        }

        if all_done {
            return task::Poll::Ready(());
        }

        task::Poll::Pending
    }
}