use std::marker;
use std::any;
use std::ptr;
use std::ops;
use hashbrown::HashMap;

/// Shared resources in the ECS.
pub struct Resources {
    data: HashMap<any::TypeId, Box<any::Any>>,
}

impl Resources {
    /// Create a new collection of resources.
    pub fn new() -> Resources {
        Resources {
            data: HashMap::new(),
        }
    }

    /// Add the given resource.
    pub fn add_resource<T>(&mut self, resource: T) where T: 'static {
        self.data.insert(any::TypeId::of::<T>(), Box::new(resource));
    }

    /// Get the immutable resource through its type id.
    fn get<T: 'static>(&self, type_id: &any::TypeId) -> Option<&T> {
        self.data.get(type_id).and_then(|a| a.downcast_ref())
    }

    /// Get the mutable resource through its type id.
    fn get_mut<T: 'static>(&mut self, type_id: &any::TypeId) -> Option<&mut T> {
        self.data.get_mut(type_id).and_then(|a| a.downcast_mut())
    }
}

/// Indirection to access resources.
///
/// This must live in a memory location that may _never move_, because it can be stored locally inside of futures.ResourceSpecification
/// 
/// Before a future is woken up, this must point to a valid set of resources.
pub struct ResourceReference {
    resources: Option<ptr::NonNull<Resources>>,
}

impl ResourceReference {
    pub fn new() -> Box<ResourceReference> {
        Box::new(ResourceReference {
            resources: None,
        })
    }

    /// Set the resources indirection.
    pub unsafe fn set_resources(&mut self, p: ptr::NonNull<Resources>) {
        self.resources = Some(p);
    }
 
    /// Clear the resources indirection.
    pub unsafe fn clear_resources(&mut self) {
        self.resources = None;
    }

    /// Get the immutable resource through its type id.
    unsafe fn get<T: 'static>(&self, type_id: &any::TypeId) -> Option<&T> {
        let resources = self.resources.as_ref().expect("resources is not set").as_ref();
        resources.get(type_id)
    }

    /// Get the mutable resource through its type id.
    unsafe fn get_mut<T: 'static>(&mut self, type_id: &any::TypeId) -> Option<&mut T> {
        let resources = self.resources.as_mut().expect("resources is not set").as_mut();
        resources.get_mut(type_id)
    }
}

pub struct ReadSpecification {
    type_id: any::TypeId,
}

pub struct WriteSpecification {
    type_id: any::TypeId,
}

pub trait Receiver<T> {
    /// Receive the given value.
    fn receive(&mut self, value: T);
}

impl<T> Receiver<T> for Vec<T> {
    fn receive(&mut self, value: T) {
        self.push(value);
    }
}

pub trait ResourceSpecification {
    /// Resources that needs to have read access.
    fn reads(_: &mut impl Receiver<ReadSpecification>) {
    }

    /// Resources that needs write access.
    fn writes(_: &mut impl Receiver<WriteSpecification>) {
    }

    /// Creates a new handle for the given resource specification.
    fn fetch(resources: ptr::NonNull<ResourceReference>) -> Self;
}

/// A resource specification indicating we want read access to the type `T`.
pub struct Read<'dispatcher, T> {
    resources: ptr::NonNull<ResourceReference>,
    type_id: any::TypeId,
    marker: marker::PhantomData<&'dispatcher T>,
}

impl<T: 'static> ResourceSpecification for Read<'_, T> {
    fn reads(out: &mut impl Receiver<ReadSpecification>) {
        out.receive(ReadSpecification {
            type_id: any::TypeId::of::<T>(),
        })
    }

    fn fetch(resources: ptr::NonNull<ResourceReference>) -> Self {
        Self {
            resources,
            type_id: any::TypeId::of::<T>(),
            marker: marker::PhantomData,
        }
    }
}

impl<'dispatcher, T> ops::Deref for Read<'_, T> where T: 'static {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            self.resources.as_ref().get(&self.type_id).expect("no such resource")
        }
    } 
}

/// A resource specification indicating that we want write access to the type `T`.
pub struct Write<'dispatcher, T> {
    resources: ptr::NonNull<ResourceReference>,
    type_id: any::TypeId,
    marker: marker::PhantomData<&'dispatcher T>,
}

impl<'dispatcher, T> ops::Deref for Write<'_, T> where T: 'static {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            self.resources.as_ref().get(&self.type_id).expect("no such resource")
        }
    } 
}

impl<'dispatcher, T> ops::DerefMut for Write<'_, T> where T: 'static {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            self.resources.as_mut().get_mut(&self.type_id).expect("no such resource")
        }
    } 
}

impl<T: 'static> ResourceSpecification for Write<'_, T> {
    fn writes(out: &mut impl Receiver<WriteSpecification>) {
        out.receive(WriteSpecification {
            type_id: any::TypeId::of::<T>(),
        })
    }

    fn fetch(resources: ptr::NonNull<ResourceReference>) -> Self {
        Self {
            resources,
            type_id: any::TypeId::of::<T>(),
            marker: marker::PhantomData,
        }
    }
}

macro_rules! impl_tuple {
    ($($name:ident),*) => {
        impl<$($name,)*> ResourceSpecification for ($($name,)*)
            where $($name: ResourceSpecification,)*
        {
            fn reads(out: &mut impl Receiver<ReadSpecification>) {
                $($name::reads(out);)*
            }

            fn writes(out: &mut impl Receiver<WriteSpecification>) {
                $($name::writes(out);)*
            }

            fn fetch(resources: ptr::NonNull<ResourceReference>) -> Self {
                (
                    $($name::fetch(resources.clone()),)*
                )
            }
        }
    }
}

impl_tuple!(A);
impl_tuple!(A, B);