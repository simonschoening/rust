use crate::convert::{TryInto,TryFrom};
use crate::fmt;
use crate::io::{self, ErrorKind, IoSlice, IoSliceMut};
use crate::net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr};
use crate::str;
use crate::sync::Arc;
use crate::sys::hermit::abi;
use crate::sys::unsupported;
use crate::os::hermit::io::{FromAbi,AsAbi};
use crate::sys_common::{FromInner,AsInner,IntoInner};
use crate::time::Duration;

/// Checks whether the HermitCore's socket interface has been started already, and
/// if not, starts it.
pub fn init() -> io::Result<()> {
    if abi::network_init() < 0 {
        return Err(io::Error::new_const(
            ErrorKind::Uncategorized,
            &"Unable to initialize network interface",
        ));
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Socket(abi::net::Socket);

impl FromInner<abi::net::Socket> for Socket {
    fn from_inner(inner: abi::net::Socket) -> Self {
        Self(inner)
    }
}

impl AsInner<abi::net::Socket> for Socket {
    fn as_inner(&self) -> &abi::net::Socket {
        &self.0
    }
}

impl IntoInner<abi::net::Socket> for Socket {
    fn into_inner(self) -> abi::net::Socket {
        let inner = self.as_inner().clone();
        crate::mem::forget(self);
        inner
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        let _ = unsafe { abi::net::socket_close(self.0) };
    }
}

// Arc is used to count the number of used sockets.
// Only if all sockets are released, the drop
// method will close the socket.
#[derive(Debug,Clone)]
pub struct TcpStream(Arc<Socket>);

impl TcpStream {
    pub fn from_socket(socket: abi::net::Socket) -> Self {
        Self(Arc::new(Socket::from_inner(socket)))
    }

    pub fn socket(&self) -> abi::net::Socket {
        self.0
            .as_inner()
            .clone()
    }

    pub fn into_socket(self) -> abi::net::Socket {
        Arc::try_unwrap(self.0)
            .unwrap()
            .into_inner()
    }

    pub fn connect(addr: io::Result<&SocketAddr>) -> io::Result<TcpStream> {
        let addr = addr?;

        let socket = unsafe { abi::net::socket() }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        unsafe { abi::net::tcp_bind(socket, abi::net::SocketAddr::V4(abi::net::SocketAddrV4::UNSPECIFIED)) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        unsafe { abi::net::tcp_connect(socket,addr.as_abi()) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        Ok(Self::from_socket(socket))
    }

    pub fn connect_timeout(saddr: &SocketAddr, duration: Duration) -> io::Result<TcpStream> {
        let socket = unsafe { abi::net::socket() }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        unsafe { abi::net::socket_set_timeout(socket, Some(duration)) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;
        
        unsafe { abi::net::tcp_bind(socket, abi::net::SocketAddr::V4(abi::net::SocketAddrV4::UNSPECIFIED)) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        unsafe { abi::net::tcp_connect(socket,saddr.as_abi()) } 
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        Ok(Self::from_socket(socket))
    }

    pub fn set_read_timeout(&self, duration: Option<Duration>) -> io::Result<()> {
        unsafe { abi::net::socket_set_timeout(self.socket(),duration) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn set_write_timeout(&self, duration: Option<Duration>) -> io::Result<()> {
        unsafe { abi::net::socket_set_timeout(self.socket(),duration) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        unsafe { abi::net::socket_timeout(self.socket()) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        unsafe { abi::net::socket_timeout(self.socket()) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn peek(&self, buffer: &mut [u8]) -> io::Result<usize> {
        unsafe { abi::net::tcp_peek(self.socket(),buffer) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        unsafe { abi::net::tcp_read(self.socket(),buffer) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn read_vectored(&self, ioslice: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        let mut empty = IoSliceMut::new(&mut []);
        let buffer = ioslice
            .iter_mut()
            .find(|slice| !slice.is_empty())
            .unwrap_or(&mut empty);
        self.read(buffer)
    }

    #[inline]
    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn write(&self, buffer: &[u8]) -> io::Result<usize> {
        unsafe { abi::net::tcp_write(self.socket(),buffer) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn write_vectored(&self, ioslice: &[IoSlice<'_>]) -> io::Result<usize> {
        let empty = IoSlice::new(&[]);
        let buffer = ioslice
            .iter()
            .find(|slice| !slice.is_empty())
            .unwrap_or(&empty);
        self.write(buffer)
    }

    #[inline]
    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        unsafe { abi::net::tcp_remote_addr(self.socket()) }
            .map(|addr| unsafe { SocketAddr::from_abi(addr) })
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        unsafe { abi::net::tcp_local_addr(self.socket()) }
            .map(|addr| unsafe { SocketAddr::from_abi(addr) })
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        unsafe { abi::net::tcp_shutdown(self.socket(), how.as_abi()) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn duplicate(&self) -> io::Result<TcpStream> {
        Ok(self.clone())
    }

    pub fn set_linger(&self, _linger: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn linger(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn set_nodelay(&self, _mode: bool) -> io::Result<()> {
        Ok(())
    }

    pub fn nodelay(&self) -> io::Result<bool> {
        Ok(true)
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        let ttl: Option<u8> = ttl.try_into().ok();
        unsafe { abi::net::tcp_set_hop_limit(self.socket(),ttl) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn ttl(&self) -> io::Result<u32> {
        unsafe { abi::net::tcp_hop_limit(self.socket()) }
            .map(|ttl| ttl.map(u32::from).unwrap_or(u32::MAX))
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        unsupported()
    }

    pub fn set_nonblocking(&self, mode: bool) -> io::Result<()> {
        unsafe { abi::net::socket_set_non_blocking(self.socket(),mode) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }
}

#[derive(Clone)]
pub struct TcpListener(Arc<Socket>);

impl TcpListener {
    pub fn from_socket(socket: abi::net::Socket) -> Self {
        Self(Arc::new(Socket::from_inner(socket)))
    }

    pub fn socket(&self) -> abi::net::Socket {
        self.0
            .as_inner()
            .clone()
    }

    pub fn into_socket(self) -> abi::net::Socket {
        Arc::try_unwrap(self.0)
            .unwrap()
            .into_inner()
    }

    pub fn bind(addr: io::Result<&SocketAddr>) -> io::Result<TcpListener> {
        let addr = addr?;

        let socket = unsafe { abi::net::socket() }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        unsafe { abi::net::tcp_bind(socket, addr.as_abi()) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        unsafe { abi::net::tcp_listen(socket,16) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        Ok(Self::from_socket(socket))
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        unsafe { abi::net::tcp_local_addr(self.socket()) }
            .map(|addr| unsafe { SocketAddr::from_abi(addr) })
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let socket = unsafe { abi::net::tcp_accept(self.socket()) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })?;

        let stream = TcpStream::from_socket(socket);
        let remote = stream.socket_addr()?;
        Ok((stream,remote))
    }

    pub fn duplicate(&self) -> io::Result<TcpListener> {
        Ok(self.clone())
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn ttl(&self) -> io::Result<u32> {
        unsupported()
    }

    pub fn set_only_v6(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn only_v6(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        unsupported()
    }

    pub fn set_nonblocking(&self, mode: bool) -> io::Result<()> {
        unsafe { abi::net::socket_set_non_blocking(self.socket(),mode) }
            .map_err(|err| unsafe { io::Error::from_abi(err) })
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

pub struct UdpSocket(abi::Handle);

impl UdpSocket {
    pub fn bind(_: io::Result<&SocketAddr>) -> io::Result<UdpSocket> {
        unsupported()
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        unsupported()
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        unsupported()
    }

    pub fn recv_from(&self, _: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        unsupported()
    }

    pub fn peek_from(&self, _: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        unsupported()
    }

    pub fn send_to(&self, _: &[u8], _: &SocketAddr) -> io::Result<usize> {
        unsupported()
    }

    pub fn duplicate(&self) -> io::Result<UdpSocket> {
        unsupported()
    }

    pub fn set_read_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn set_write_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        unsupported()
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        unsupported()
    }

    pub fn set_broadcast(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn broadcast(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn set_multicast_loop_v4(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn multicast_loop_v4(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn set_multicast_ttl_v4(&self, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn multicast_ttl_v4(&self) -> io::Result<u32> {
        unsupported()
    }

    pub fn set_multicast_loop_v6(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn multicast_loop_v6(&self) -> io::Result<bool> {
        unsupported()
    }

    pub fn join_multicast_v4(&self, _: &Ipv4Addr, _: &Ipv4Addr) -> io::Result<()> {
        unsupported()
    }

    pub fn join_multicast_v6(&self, _: &Ipv6Addr, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn leave_multicast_v4(&self, _: &Ipv4Addr, _: &Ipv4Addr) -> io::Result<()> {
        unsupported()
    }

    pub fn leave_multicast_v6(&self, _: &Ipv6Addr, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        unsupported()
    }

    pub fn ttl(&self) -> io::Result<u32> {
        unsupported()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        unsupported()
    }

    pub fn set_nonblocking(&self, _: bool) -> io::Result<()> {
        unsupported()
    }

    pub fn recv(&self, _: &mut [u8]) -> io::Result<usize> {
        unsupported()
    }

    pub fn peek(&self, _: &mut [u8]) -> io::Result<usize> {
        unsupported()
    }

    pub fn send(&self, _: &[u8]) -> io::Result<usize> {
        unsupported()
    }

    pub fn connect(&self, _: io::Result<&SocketAddr>) -> io::Result<()> {
        unsupported()
    }
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

pub struct LookupHost(!);

impl LookupHost {
    pub fn port(&self) -> u16 {
        self.0
    }
}

impl Iterator for LookupHost {
    type Item = SocketAddr;
    fn next(&mut self) -> Option<SocketAddr> {
        self.0
    }
}

impl TryFrom<&str> for LookupHost {
    type Error = io::Error;

    fn try_from(_v: &str) -> io::Result<LookupHost> {
        unsupported()
    }
}

impl<'a> TryFrom<(&'a str, u16)> for LookupHost {
    type Error = io::Error;

    fn try_from(_v: (&'a str, u16)) -> io::Result<LookupHost> {
        unsupported()
    }
}

#[allow(nonstandard_style)]
pub mod netc {
    pub const AF_INET: u8 = 0;
    pub const AF_INET6: u8 = 1;
    pub type sa_family_t = u8;

    #[derive(Copy, Clone)]
    pub struct in_addr {
        pub s_addr: u32,
    }

    #[derive(Copy, Clone)]
    pub struct sockaddr_in {
        pub sin_family: sa_family_t,
        pub sin_port: u16,
        pub sin_addr: in_addr,
    }

    #[derive(Copy, Clone)]
    pub struct in6_addr {
        pub s6_addr: [u8; 16],
    }

    #[derive(Copy, Clone)]
    pub struct sockaddr_in6 {
        pub sin6_family: sa_family_t,
        pub sin6_port: u16,
        pub sin6_addr: in6_addr,
        pub sin6_flowinfo: u32,
        pub sin6_scope_id: u32,
    }

    #[derive(Copy, Clone)]
    pub struct sockaddr {}

    pub type socklen_t = usize;
}
