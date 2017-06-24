use futures::Stream;
use error::*;
use std::result::Result as StdResult;
use tokio_core::reactor::Handle;

pub trait Component {
    type Stream: Stream<Item=String, Error=Self::Error> + 'static;
    type Error: ::std::error::Error + Send + 'static;

    fn init(&mut self) -> StdResult<(), Self::Error> { Ok(()) }
    fn stream(self, handle: Handle) -> Self::Stream;
}

pub trait ComponentCreator {
    fn init(&mut self) -> StdResult<(), Error>;
    fn into_stream(self: Box<Self>, handle: Handle) -> Box<Stream<Item=String, Error=Error>>;
    fn create(mut self: Box<Self>, handle: Handle)
        -> StdResult<Box<Stream<Item=String, Error=Error>>, Error>
    {
        self.init()?;
        Ok(self.into_stream(handle))
    }
}

impl<C> ComponentCreator for C
    where C: Component,
{
    fn init(&mut self) -> StdResult<(), Error> {
        Component::init(self)
            .chain_err(|| "Failed to initialize component")
    }

    fn into_stream(self: Box<Self>, handle: Handle)
        -> Box<Stream<Item=String, Error=Error>>
    {
        Box::new(self.stream(handle)
           .map_err(|e| Error::with_chain(e, "Component raised an error")))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Slot {
    Left,
    Right,
    Center,
}

pub struct ComponentUpdate {
    pub slot: Slot,
    pub index: usize,
    pub id: usize,
    pub value: String,
}
