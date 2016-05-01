use net2::{TcpBuilder, TcpStreamExt};
#[cfg(unix)] use unix_socket::{UnixStream};

use std::time::Duration;
use std::io;
use std::net::{SocketAddr};
use std::sync::{Arc, Mutex};

use protocol::stream::Stream;

use ::proto::auth::client::AuthError;
use ::proto::content::client::ContentInterface;

use super::config::{Config, Target};
use super::connection::{Connection};



#[derive(Debug)]
pub enum ConnectError {
    TCP(io::Error),
    Unixsocket(io::Error),
    Unsupported,
    AuthError(AuthError)
}



/// The database client
pub struct Client {
    config: Config,
//    connection: Arc<Mutex<Option<Connection>>>,
//    stream_thread: Option<thread::JoinHandle<()>>,
//    /// The client unique identifier
//    id: Option<usize>,
    /// An incremental id for new tasks
    pub next_id: Arc<Mutex<usize>>,
}



impl Client {
    /// Creates a new client
    pub fn new(config: Config) -> Client {
        return Client {
            config: config,
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Starts thread communication with server.
    pub fn connect(&mut self) -> Result<ContentInterface, ConnectError> {
        match self.config.target.clone() {
            Target::Unix(unixsocket) => self.connect_unixsocket(unixsocket),
            Target::Tcp(address) => self.connect_tcp(address),
        }
    }

//    /// Sends a kill signal to the listeners and connects to the incoming
//    /// connections to break the listening loop.
//    // TODO
//    pub fn send(&mut self) {
//        #![allow(unused_must_use)]
//        let sender = self.stream_channel.unwrap();
//        sender.send(0);
//        self.stream.shutdown();
//        self.join();
//    }

    /// Connects to a socket address.
    fn connect_tcp(&mut self, addr: SocketAddr) -> Result<ContentInterface, ConnectError> {
        let (tcp_keepalive, timeout) = {
            let c = &self.config;
            (c.tcp_keepalive, c.timeout)
        };

        let tcp_builder = match match addr {
            SocketAddr::V4(_) => TcpBuilder::new_v4(),
            SocketAddr::V6(_) => TcpBuilder::new_v6(),
        } {
            Ok(v) => v,
            Err(err) => return Err(ConnectError::TCP(err))
        };

        let stream = match tcp_builder.connect(addr) {
            Ok(s) => s,
            Err(err) => return Err(ConnectError::TCP(err))
        };

        stream.set_keepalive(
            if tcp_keepalive > 0 { Some(Duration::from_secs(tcp_keepalive as u64)) }
            else { None }
        ).unwrap();
        stream.set_read_timeout(
            if timeout > 0 { Some(Duration::new(timeout, 0)) }
            else { None }
        ).unwrap();
        stream.set_write_timeout(
            if timeout > 0 { Some(Duration::new(timeout, 0)) }
            else { None }
        ).unwrap();

        match Connection::connect(Stream::Tcp(stream)) {
            Ok(protocol) => Ok(protocol),
            Err(error) => Err(ConnectError::AuthError(error))
        }
    }

    #[cfg(unix)]
    fn connect_unixsocket(&mut self, unixsocket: String) -> Result<ContentInterface, ConnectError> {
        let stream = match UnixStream::connect(unixsocket) {
            Ok(s) => s,
            Err(err) => return Err(ConnectError::Unixsocket(err))
        };

        match Connection::connect(Stream::Unix(stream)) {
            Ok(protocol) => Ok(protocol),
            Err(error) => Err(ConnectError::AuthError(error))
        }
    }

    #[cfg(not(unix))]
    fn connect_unixsocket(&mut self, unixsocket: String) -> Result<ContentInterface, ConnectError> {
        Err(ConnectError::Unsupported)
    }

//    /// Sends a kill signal to the listeners and connects to the incoming
//    /// connections to break the listening loop.
//    // TODO
//    pub fn disconnect(&mut self) {
//        #![allow(unused_must_use)]
//        let sender = self.stream_channel;
//        sender.send(0);
//        self.stream.shutdown();
//        self.join();
//    }

//    /// Join the listener threads.
//    pub fn join(&mut self) {
//        #![allow(unused_must_use)]
//        self.listener_thread.join();
//    }
}
