// SPDX-License-Identifier: GPL-3.0-or-later
//! Audited socket boundary: address-taking operations accept only SocketAddr.
use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};

pub struct NumericUdp(UdpSocket);

impl NumericUdp {
    pub fn bind(address: SocketAddr) -> io::Result<Self> {
        UdpSocket::bind(address).map(Self)
    }

    pub fn sender(peer: SocketAddr) -> io::Result<Self> {
        let local = if peer.is_ipv4() {
            SocketAddr::from(([0, 0, 0, 0], 0))
        } else {
            SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0))
        };
        let socket = Self::bind(local)?;
        socket.0.connect(peer)?;
        Ok(socket)
    }

    pub fn recv_from(&self, bytes: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(bytes)
    }

    pub fn set_read_timeout(&self, timeout: Option<std::time::Duration>) -> io::Result<()> {
        self.0.set_read_timeout(timeout)
    }

    pub fn send(&self, bytes: &[u8]) -> io::Result<usize> {
        self.0.send(bytes)
    }
}
