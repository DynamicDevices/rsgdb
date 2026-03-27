//! Byte stream to the debug target (TCP stub today; more variants later).

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

/// Async byte stream used as the GDB remote **target** side of the proxy.
#[derive(Debug)]
pub enum BackendStream {
    /// Remote GDB stub (OpenOCD, probe-rs, gdbserver, …) over TCP.
    Tcp(TcpStream),
}

impl AsyncRead for BackendStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match &mut *self {
            BackendStream::Tcp(t) => Pin::new(t).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for BackendStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match &mut *self {
            BackendStream::Tcp(t) => Pin::new(t).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match &mut *self {
            BackendStream::Tcp(t) => Pin::new(t).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match &mut *self {
            BackendStream::Tcp(t) => Pin::new(t).poll_shutdown(cx),
        }
    }
}
