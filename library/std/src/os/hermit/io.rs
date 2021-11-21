#![stable(feature = "rust1", since = "1.0.0")]

use crate::os::hermit::abi;
use crate::sys::net;
use crate::sys_common::{AsInner,FromInner,IntoInner};
use crate::net::{TcpListener,TcpStream,UdpSocket};
use crate::net::Shutdown;
use crate::net::{IpAddr,Ipv4Addr,Ipv6Addr};
use crate::net::{SocketAddr,SocketAddrV4,SocketAddrV6};
use crate::io::{Error,ErrorKind};

/// convert a hermit-abi type into a std type
#[stable(feature = "rust1", since = "1.0.0")]
pub trait FromAbi {
    #[stable(feature = "rust1", since = "1.0.0")]
    type AbiType;
    #[stable(feature = "rust1", since = "1.0.0")]
    unsafe fn from_abi(abi_type: Self::AbiType) -> Self;
}

/// convert a std type into a hermit-abi type without passing ownership
#[stable(feature = "rust1", since = "1.0.0")]
pub trait AsAbi {
    #[stable(feature = "rust1", since = "1.0.0")]
    type AbiType;
    #[stable(feature = "rust1", since = "1.0.0")]
    fn as_abi(&self) -> Self::AbiType;
}

/// convert a std type into a hermit-abi type while passing ownership
#[stable(feature = "rust1", since = "1.0.0")]
pub trait IntoAbi {
    #[stable(feature = "rust1", since = "1.0.0")]
    type AbiType;
    #[stable(feature = "rust1", since = "1.0.0")]
    fn into_abi(self) -> Self::AbiType;
}

macro_rules! impl_from_abi {
    ( $abi_type:ty => $std_type:ty |$ident:ident| $block:block ) => {
        #[stable(feature = "rust1", since = "1.0.0")]
        impl FromAbi for $std_type {
            type AbiType = $abi_type;
            unsafe fn from_abi($ident: Self::AbiType) -> Self {
                $block
            }
        }
    }
}

macro_rules! impl_as_abi {
    ( $std_type:ty => $abi_type:ty |$slf:ident| $block:block ) => {
        #[stable(feature = "rust1", since = "1.0.0")]
        impl AsAbi for $std_type {
            type AbiType = $abi_type;
            fn as_abi(&$slf) -> Self::AbiType {
                $block
            }
        }
    }
}

macro_rules! impl_into_abi {
    ( $std_type:ty => $abi_type:ty |$slf:ident| $block:block ) => {
        #[stable(feature = "rust1", since = "1.0.0")]
        impl IntoAbi for $std_type {
            type AbiType = $abi_type;
            fn into_abi($slf) -> Self::AbiType {
                $block
            }
        }
    }
}

// ip address

impl_from_abi!{ abi::net::Ipv4Addr => Ipv4Addr
    |ip| { Ipv4Addr::new(ip.a,ip.b,ip.c,ip.d) }
}

impl_from_abi!{ abi::net::Ipv6Addr => Ipv6Addr
    |ip| { Ipv6Addr::new(ip.a,ip.b,ip.c,ip.d,ip.e,ip.f,ip.g,ip.h) }
}

impl_from_abi!{ abi::net::IpAddr => IpAddr
    |ip| { 
        match ip {
            abi::net::IpAddr::V4(ipv4) => IpAddr::V4(Ipv4Addr::from_abi(ipv4)),
            abi::net::IpAddr::V6(ipv6) => IpAddr::V6(Ipv6Addr::from_abi(ipv6)),
        }
    }
}

impl_as_abi!{ Ipv4Addr => abi::net::Ipv4Addr
    |self| {
        let [a,b,c,d] = self.octets();
        abi::net::Ipv4Addr { a,b,c,d }
    }
}

impl_as_abi!{ Ipv6Addr => abi::net::Ipv6Addr
    |self| {
        let [a,b,c,d,e,f,g,h] = self.segments();
        abi::net::Ipv6Addr { a,b,c,d,e,f,g,h }
    }
}

impl_as_abi!{ IpAddr => abi::net::IpAddr
    |self| {
        match self {
            IpAddr::V4(ipv4) => abi::net::IpAddr::V4(ipv4.as_abi()),
            IpAddr::V6(ipv6) => abi::net::IpAddr::V6(ipv6.as_abi()),
        }
    }
}

// socket address

impl_from_abi!{ abi::net::SocketAddrV4 => SocketAddrV4
    |saddr| { SocketAddrV4::new(Ipv4Addr::from_abi(saddr.ip_addr),saddr.port) }
}

impl_from_abi!{ abi::net::SocketAddrV6 => SocketAddrV6
    |saddr| { SocketAddrV6::new(Ipv6Addr::from_abi(saddr.ip_addr),saddr.port,saddr.flowinfo,saddr.scope_id) }
}

impl_from_abi!{ abi::net::SocketAddr => SocketAddr
    |saddr| {
        match saddr {
            abi::net::SocketAddr::V4(saddr4) => SocketAddr::V4(SocketAddrV4::from_abi(saddr4)),
            abi::net::SocketAddr::V6(saddr6) => SocketAddr::V6(SocketAddrV6::from_abi(saddr6)),
        }
    }
}

impl_as_abi!{ SocketAddrV4 => abi::net::SocketAddrV4
    |self| {
        abi::net::SocketAddrV4 { 
            ip_addr: self.ip().as_abi(), 
            port: self.port(),
        }
    }
}

impl_as_abi!{ SocketAddrV6 => abi::net::SocketAddrV6
    |self| {
        abi::net::SocketAddrV6 { 
            ip_addr: self.ip().as_abi(), 
            port: self.port(),
            flowinfo: self.flowinfo(),
            scope_id: self.scope_id(),
        }
    }
}

impl_as_abi!{ SocketAddr => abi::net::SocketAddr
    |self| {
        match self {
            SocketAddr::V4(saddr4) => abi::net::SocketAddr::V4(saddr4.as_abi()),
            SocketAddr::V6(saddr4) => abi::net::SocketAddr::V6(saddr4.as_abi()),
        }
    }
}

// shutdown

impl_from_abi!{ abi::net::Shutdown => Shutdown
    |sd| {
        match sd {
            abi::net::Shutdown::Read => Shutdown::Read,
            abi::net::Shutdown::Write => Shutdown::Write,
            abi::net::Shutdown::Both => Shutdown::Both,
        }
    }
}

impl_as_abi!{ Shutdown => abi::net::Shutdown
    |self| { 
        match self {
            Shutdown::Read => abi::net::Shutdown::Read,
            Shutdown::Write => abi::net::Shutdown::Write,
            Shutdown::Both => abi::net::Shutdown::Both,
        }
    }
}

// tcp stream/listener

impl_from_abi!{ abi::net::Socket => TcpStream
    |socket| { TcpStream::from_inner(net::TcpStream::from_socket(socket)) }
}

impl_as_abi!{ TcpStream => abi::net::Socket
    |self| { self.as_inner().socket() }
}

impl_into_abi!{ TcpStream => abi::net::Socket
    |self| { self.into_inner().into_socket() }
}

impl_from_abi!{ abi::net::Socket => TcpListener
    |socket| { TcpListener::from_inner(net::TcpListener::from_socket(socket)) }
}

impl_as_abi!{ TcpListener => abi::net::Socket
    |self| { self.as_inner().socket() }
}

impl_into_abi!{ TcpListener => abi::net::Socket
    |self| { self.into_inner().into_socket() }
}

// udp socket

impl_from_abi!{ abi::net::Socket => UdpSocket
    |_socket| { unimplemented!() }
}

impl_as_abi!{ UdpSocket => abi::net::Socket
    |self| { unimplemented!() }
}

impl_into_abi!{ UdpSocket => abi::net::Socket
    |self| { unimplemented!() }
}


// io Error

impl_from_abi!{ abi::io::ErrorKind => ErrorKind
    |kind| {
        match kind {
            abi::io::ErrorKind::AlreadyExists => ErrorKind::AlreadyExists,
            abi::io::ErrorKind::NotSocket => ErrorKind::InvalidInput,
            abi::io::ErrorKind::NotFound => ErrorKind::NotFound,
            abi::io::ErrorKind::NotListening => ErrorKind::InvalidData,
            abi::io::ErrorKind::InUse => ErrorKind::InvalidData,
            abi::io::ErrorKind::ConnectionRefused => ErrorKind::ConnectionRefused,
            abi::io::ErrorKind::ConnectionReset => ErrorKind::ConnectionReset,
            abi::io::ErrorKind::NotConnected => ErrorKind::NotConnected,
            abi::io::ErrorKind::AddrInUse => ErrorKind::AddrInUse,
            abi::io::ErrorKind::AddrNotAvailable => ErrorKind::AddrNotAvailable,
            abi::io::ErrorKind::WouldBlock => ErrorKind::WouldBlock,
            abi::io::ErrorKind::InvalidInput => ErrorKind::InvalidInput,
            abi::io::ErrorKind::InvalidData => ErrorKind::InvalidData,
            abi::io::ErrorKind::TimedOut => ErrorKind::TimedOut,
            abi::io::ErrorKind::WriteZero => ErrorKind::WriteZero,
            abi::io::ErrorKind::Other => ErrorKind::Other,
            abi::io::ErrorKind::Unsupported => ErrorKind::Unsupported,
            _ => ErrorKind::Other,
        }
    }
}

impl_from_abi!{ abi::io::Error => Error
    |err| { Error::new_const(ErrorKind::from_abi(err.kind),err.msg) }
}
