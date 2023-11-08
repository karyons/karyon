mod connection;
mod endpoint;
mod error;
mod listener;
mod transports;

pub use {
    connection::{dial, Conn, Connection},
    endpoint::{Addr, Endpoint, Port},
    listener::{listen, Listener},
    transports::{
        tcp::{dial_tcp, listen_tcp, TcpConn},
        udp::{dial_udp, listen_udp, UdpConn},
        unix::{dial_unix, listen_unix, UnixConn},
    },
};

use error::{Error, Result};

/// Represents Karyons's Net Error
pub use error::Error as NetError;

/// Represents Karyons's Net Result
pub use error::Result as NetResult;
