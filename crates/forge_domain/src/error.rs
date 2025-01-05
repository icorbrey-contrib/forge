use std::pin::Pin;

use derive_more::derive::From;

#[derive(From, Debug)]
pub enum Error {
    ToolUseMissingName,
    Serde(serde_json::Error),
}

pub type Result<A> = std::result::Result<A, Error>;
pub type BoxStream<A, E> =
    Pin<Box<dyn tokio_stream::Stream<Item = std::result::Result<A, E>> + Send>>;
pub type ResultStream<A, E> = std::result::Result<BoxStream<A, E>, E>;
