//! This module contains error object for PromptBuffer
use std::io;
use std::convert;
use std::sync::mpsc;

/// Convenience wrapper for Result<T, PromptBufferError>
pub type PromptBufferResult<T> = Result<T, PromptBufferError>;

/// The base error type of PromptBuffer
pub enum PromptBufferError {
    /// Error variant for IO errors
    IO(io::Error),

    /// Error variant for channel send errors
    SendError(mpsc::SendError<()>),
}

macro_rules! convert_impl {
    ($($from:ty => $to:ident),+) => {$(
        impl convert::From<$from> for PromptBufferError {
            fn from(error: $from) -> PromptBufferError {
                PromptBufferError::$to(error)
            }
        }
    )+}
}

convert_impl! {
    io::Error => IO,
    mpsc::SendError<()> => SendError
}
