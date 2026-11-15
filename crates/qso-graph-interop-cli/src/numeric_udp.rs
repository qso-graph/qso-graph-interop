// SPDX-License-Identifier: GPL-3.0-or-later
//! Audited socket boundary: numeric destinations and explicit multicast interfaces only.
#![forbid(unsafe_code)]
use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

pub type Interface = Ipv4Addr;

fn validate_interface(interface: Interface) -> io::Result<()> {
    if interface.is_unspecified() || interface.is_multicast() || interface.is_broadcast() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--interface must be a numeric local IPv4 address other than 0.0.0.0, multicast, or 255.255.255.255",
        ));
    }
    Ok(())
}

pub fn interface(value: &str) -> Result<Interface, String> {
    let interface = value
        .parse()
        .map_err(|_| "--interface must be a numeric local IPv4 address".to_owned())?;
    validate_interface(interface).map_err(|error| error.to_string())?;
    Ok(interface)
}

pub fn validate(address: SocketAddr, interface: Option<Interface>) -> io::Result<()> {
    if let IpAddr::V6(group) = address.ip()
        && group.is_multicast()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "IPv6 multicast is not supported; use an IPv4 group",
        ));
    }
    match (address.ip().is_multicast(), interface) {
        (true, Some(interface)) => validate_interface(interface),
        (true, None) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "an IPv4 multicast group requires --interface <IPV4>; supply a numeric local interface address",
        )),
        (false, Some(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--interface requires an IPv4 multicast address; use a multicast group or remove --interface",
        )),
        (false, None) => Ok(()),
    }
}

pub struct NumericUdp(UdpSocket);

impl NumericUdp {
    pub fn bind(address: SocketAddr, interface: Option<Interface>) -> io::Result<Self> {
        validate(address, interface)?;
        if let (IpAddr::V4(group), Some(interface)) = (address.ip(), interface) {
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
            socket.set_reuse_address(true)?;
            // Windows cannot bind a multicast group; other targets filter by group at bind.
            let local = if cfg!(windows) {
                SocketAddr::from((Ipv4Addr::UNSPECIFIED, address.port()))
            } else {
                address
            };
            socket.bind(&local.into())?;
            socket.join_multicast_v4(&group, &interface)?;
            return Ok(Self(socket.into()));
        }
        UdpSocket::bind(address).map(Self)
    }

    pub fn sender(peer: SocketAddr, interface: Option<Interface>) -> io::Result<Self> {
        validate(peer, interface)?;
        if let Some(interface) = interface {
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
            socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)).into())?;
            socket.set_multicast_if_v4(&interface)?;
            socket.set_multicast_ttl_v4(1)?;
            socket.set_multicast_loop_v4(true)?;
            socket.connect(&peer.into())?;
            return Ok(Self(socket.into()));
        }
        let local = if peer.is_ipv4() {
            SocketAddr::from(([0, 0, 0, 0], 0))
        } else {
            SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0))
        };
        let socket = Self::bind(local, None)?;
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
