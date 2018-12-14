use std::pin::Pin;
use std::future::Future;
use crate::resources::{ResourceSpecification};

/// The system trait is the central abstraction of our ECS.
/// 
/// A system is responsible for defining a single behavior.
pub trait System<'future> {
    type Data: ResourceSpecification;

    fn run(self, data: Self::Data) -> Pin<Box<dyn Future<Output = ()> + 'future>>;
}