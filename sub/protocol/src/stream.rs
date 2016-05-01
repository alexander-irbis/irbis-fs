use std::io;
use std::io::prelude::*;
use std::net::{TcpStream, Shutdown};
use std::time::Duration;

use net2::TcpStreamExt;
#[cfg(unix)] use unix_socket::UnixStream;


/// A stream connection.
#[cfg(unix)]
#[derive(Debug)]
pub enum Stream {
    Tcp(TcpStream),
    Unix(UnixStream),
}


#[cfg(not(unix))]
#[derive(Debug)]
pub enum Stream {
    Tcp(TcpStream),
}


// --------------------------------------------------------------------------------------------------------------------


#[cfg(unix)]
impl Stream {
    /// Creates a new independently owned handle to the underlying socket.
    pub fn try_clone(&self) -> io::Result<Stream> {
        match *self {
            Stream::Tcp(ref s) => Ok(Stream::Tcp(try!(s.try_clone()))),
            Stream::Unix(ref s) => Ok(Stream::Unix(try!(s.try_clone()))),
        }
    }

    ///
    pub fn shutdown(&self) -> io::Result<()> {
        match *self {
            Stream::Tcp(ref s) => s.shutdown(Shutdown::Both),
            Stream::Unix(ref s) => s.shutdown(Shutdown::Both),
        }
    }

    /// Write a buffer into this object, returning how many bytes were written.
    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            Stream::Tcp(ref mut s) => s.write(buf),
            Stream::Unix(ref mut s) => s.write(buf),
        }
    }

    /// Sets the keepalive timeout to the timeout specified.
    /// It fails silently for UNIX sockets.
    pub fn set_keepalive(&self, duration: Option<Duration>) -> io::Result<()> {
        match *self {
            Stream::Tcp(ref s) => TcpStreamExt::set_keepalive(s, duration),
            Stream::Unix(_) => Ok(()),
        }
    }

    /// Sets the write timeout to the timeout specified.
    /// It fails silently for UNIX sockets.
    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        match *self {
            Stream::Tcp(ref s) => s.set_write_timeout(dur),
            // TODO: couldn't figure out how to enable this in unix_socket
            Stream::Unix(_) => Ok(()),
        }
    }

    /// Sets the read timeout to the timeout specified.
    /// It fails silently for UNIX sockets.
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        match *self {
            Stream::Tcp(ref s) => s.set_read_timeout(dur),
            // TODO: couldn't figure out how to enable this in unix_socket
            Stream::Unix(_) => Ok(()),
        }
    }
}


#[cfg(unix)]
impl Read for Stream {
    /// Pull some bytes from this source into the specified buffer,
    /// returning how many bytes were read.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            Stream::Tcp(ref mut s) => s.read(buf),
            Stream::Unix(ref mut s) => s.read(buf),
        }
    }
}
