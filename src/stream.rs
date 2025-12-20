use std::{
    pin::Pin,
    task::{Context, Poll},
};

use hyper::rt::{Read, Write};
use hyper_util::client::legacy::connect::{Connected, Connection};

#[cfg(feature = "custom-transport")]
pub trait CustomGrpcStream: hyper::rt::Read + hyper::rt::Write + Send + Unpin {}

#[cfg(feature = "custom-transport")]
impl<S> CustomGrpcStream for S where S: hyper::rt::Read + hyper::rt::Write + Send + Unpin {}

pub enum GrpcStream {
    #[cfg(feature = "unix-transport")]
    Unix(hyper_util::rt::TokioIo<tokio::net::UnixStream>),
    #[cfg(feature = "custom-transport")]
    Custom(Box<dyn CustomGrpcStream>),
}

impl Read for GrpcStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            #[cfg(feature = "unix-transport")]
            GrpcStream::Unix(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(feature = "custom-transport")]
            GrpcStream::Custom(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl Write for GrpcStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            #[cfg(feature = "unix-transport")]
            GrpcStream::Unix(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(feature = "custom-transport")]
            GrpcStream::Custom(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            #[cfg(feature = "unix-transport")]
            GrpcStream::Unix(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "custom-transport")]
            GrpcStream::Custom(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            #[cfg(feature = "unix-transport")]
            GrpcStream::Unix(stream) => Pin::new(stream).poll_shutdown(cx),
            #[cfg(feature = "custom-transport")]
            GrpcStream::Custom(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

impl Connection for GrpcStream {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}
