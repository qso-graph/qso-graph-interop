// SPDX-License-Identifier: GPL-3.0-or-later
//! Audited numeric, loopback-only adapter for fldigi HTTP clients and listeners.
#![forbid(unsafe_code)]
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};
pub struct LoopbackTcp {
    stream: TcpStream,
    deadline: Instant,
}
impl LoopbackTcp {
    pub fn connect(address: SocketAddr) -> io::Result<Self> {
        if !address.ip().is_loopback() || address.port() == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fldigi XML-RPC uses plain HTTP; only numeric loopback addresses with a nonzero port are allowed",
            ));
        }
        let timeout = Duration::from_secs(5);
        let deadline = Instant::now() + timeout;
        let stream = TcpStream::connect_timeout(&address, timeout)?;
        Ok(Self { stream, deadline })
    }
    pub fn restart_deadline(&mut self) {
        self.deadline = Instant::now() + Duration::from_secs(5);
    }
    fn remaining(&self) -> io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "fldigi HTTP request exceeded five seconds",
                )
            })
    }
}
pub struct LoopbackListener(TcpListener);
impl LoopbackListener {
    pub fn bind(address: SocketAddr) -> io::Result<Self> {
        if !address.ip().is_loopback() || address.port() == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fldigi XML-RPC uses plain HTTP; only numeric loopback listeners with a nonzero port are allowed",
            ));
        }
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self(listener))
    }
    pub fn accept(&self) -> io::Result<LoopbackTcp> {
        let (stream, peer) = self.0.accept()?;
        if !peer.ip().is_loopback() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "non-loopback peer",
            ));
        }
        stream.set_nonblocking(true)?;
        Ok(LoopbackTcp {
            stream,
            deadline: Instant::now() + Duration::from_secs(5),
        })
    }
}
impl Read for LoopbackTcp {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(buf)
    }
}
impl Write for LoopbackTcp {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
