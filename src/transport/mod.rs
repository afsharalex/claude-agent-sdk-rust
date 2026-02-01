//! Transport implementations for Claude SDK.

mod subprocess;

pub use subprocess::SubprocessCLITransport;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

use crate::error::Result;

/// Abstract transport for Claude communication.
///
/// This is a low-level transport interface that handles raw I/O with the Claude
/// process or service. The Query class builds on top of this to implement the
/// control protocol and message routing.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect the transport and prepare for communication.
    ///
    /// For subprocess transports, this starts the process.
    /// For network transports, this establishes the connection.
    async fn connect(&mut self) -> Result<()>;

    /// Write raw data to the transport.
    ///
    /// # Arguments
    /// * `data` - Raw string data to write (typically JSON + newline)
    async fn write(&mut self, data: &str) -> Result<()>;

    /// Read and parse messages from the transport.
    ///
    /// Returns a stream of parsed JSON messages.
    fn read_messages(&mut self) -> Pin<Box<dyn Stream<Item = Result<Value>> + Send + '_>>;

    /// Read a single message from the transport.
    ///
    /// Returns the next available message or None if the stream ended.
    async fn read_next_message(&mut self) -> Result<Option<Value>>;

    /// Close the transport connection and clean up resources.
    async fn close(&mut self) -> Result<()>;

    /// Check if transport is ready for communication.
    fn is_ready(&self) -> bool;

    /// End the input stream (close stdin for process transports).
    async fn end_input(&mut self) -> Result<()>;
}
