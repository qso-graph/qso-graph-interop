// SPDX-License-Identifier: GPL-3.0-or-later
//! Audited numeric, loopback-only adapter for fldigi's plain HTTP XML-RPC client.
#![forbid(unsafe_code)]
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
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
