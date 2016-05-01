#[cfg(unix)] use std::fs::File;

#[cfg(unix)] use libc::fork;
#[cfg(unix)] use libc::exit;
#[cfg(unix)] use libc::getpid;
use net2::{TcpBuilder, TcpStreamExt};
#[cfg(unix)] use unix_socket::{UnixListener};

use std::time::Duration;
use std::io;
use std::io::{Write};
use std::net::{SocketAddr, ToSocketAddrs, TcpStream};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, channel};
use std::thread;

use protocol::stream::Stream;


use super::config::Config;
use super::connection::Connection;
use super::database::Database;



/// The database server
pub struct Server {
    /// A reference to the database
    pub db: Arc<Mutex<Database>>,
    /// A list of channels listening for incoming connections
    listener_channels: Vec<Sender<u8>>,
    /// A list of threads listening for incoming connections
    listener_threads: Vec<thread::JoinHandle<()>>,
    /// An incremental id for new clients
    pub next_id: Arc<Mutex<usize>>,
}



impl Server {
    /// Creates a new server
    pub fn new(config: Config) -> io::Result<Server> {
        Ok(Server {
            db: Arc::new(Mutex::new(Database::new(config)?)),
            listener_channels: Vec::new(),
            listener_threads: Vec::new(),
            next_id: Arc::new(Mutex::new(0)),
        })
    }

    /// Runs the server. If `config.daemonize` is true, it forks and exits.
    #[cfg(unix)]
    pub fn run(&mut self) {
        let (daemonize, pidfile) = {
            let db = self.db.lock().unwrap();
            (db.config.daemonize, db.config.pidfile)
        };
        if daemonize {
            unsafe {
                match fork() {
                    -1 => panic!("Fork failed"),
                    0 => {
                        if let Ok(mut fp) = File::create(&pidfile) {
                            match write!(fp, "{}", getpid()) {
                                Ok(_) => (),
                                Err(e) => {
                                    warn!("Error writing pid: {}", e);
                                },
                            }
                        }
                        self.start();
                        self.join();
                    },
                    _ => exit(0),
                };
            }
        } else {
            self.start();
            self.join();
        }
    }

    #[cfg(not(unix))]
    pub fn run(&mut self) {
        let daemonize = {
            let db = self.db.lock().unwrap();
            db.config.daemonize
        };
        if daemonize {
            panic!("Cannot daemonize in non-unix");
        } else {
            self.start();
            self.join();
        }
    }

    #[cfg(windows)]
    fn reuse_address(&self, _: &TcpBuilder) -> io::Result<()> {
        Ok(())
    }

    #[cfg(not(windows))]
    fn reuse_address(&self, builder: &TcpBuilder) -> io::Result<()> {
        try!(builder.reuse_address(true));
        Ok(())
    }

    /// Listens to a socket address.
    fn handle_tcp<T: ToSocketAddrs>(&mut self, t: T, tcp_keepalive: u32, timeout: u64, tcp_backlog: i32) -> io::Result<()> {
        for addr in try!(t.to_socket_addrs()) {
            let tcp_builder = try!(match addr {
                SocketAddr::V4(_) => TcpBuilder::new_v4(),
                SocketAddr::V6(_) => TcpBuilder::new_v6(),
            });

            try!(self.reuse_address(&tcp_builder));
            let listener = try!(try!(tcp_builder.bind(addr)).listen(tcp_backlog));
            {
                self.handle_listener(move || {
                    let stream = listener.incoming().next().unwrap();
                    match stream {
                        Err(e) => Err(e),
                        Ok(stream) => {
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
                            Ok(Stream::Tcp(stream))
                        },
                    }
                });
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn handle_unixsocket(&mut self) {
        let db = self.db.clone();
        let db = db.lock().unwrap();
        if let Some(ref unixsocket) = db.config.unixsocket {

            let listener = match UnixListener::bind(unixsocket) {
                Ok(l) => l,
                Err(err) => {
                    warn!("Creating Server Unix socket {}: {:?}", unixsocket, err);
                    return;
                }
            };
            self.handle_listener(move || {
                let stream = listener.incoming().next().unwrap();
                stream.map(|stream| Stream::Unix(stream) )
            });
        }
    }

    #[cfg(not(unix))]
    fn handle_unixsocket(&mut self) {
        let db = self.db.lock().unwrap();
        if db.config.unixsocket.is_some() {
            let _ = writeln!(&mut std::io::stderr(), "Ignoring unixsocket in non unix environment\n");
        }
    }

    fn handle_listener<F>(&mut self, incoming: F) where F: Send + 'static + Fn() -> io::Result<Stream> {
        let (tx, rx) = channel();
        self.listener_channels.push(tx);
        let db = self.db.clone();
        let next_id = self.next_id.clone();

        let th = thread::spawn(move || {
            loop {
                let stream = incoming();
                if rx.try_recv().is_ok() {
                    // any new message should break
                    break;
                }
                match stream {
                    Ok(stream) => {
                        info!("Accepted connection to {:?}", stream);
                        let id = {
                            let mut nid = next_id.lock().unwrap();
                            *nid += 1;
                            *nid - 1
                        };
                        let mut connection = Connection::new(stream, db.clone(), id);
                        thread::spawn(move || {
                            connection.run();
                        });
                    },
                    Err(e) => warn!("Accepting client connection: {:?}", e),
                }
            }
        });
        self.listener_threads.push(th);
    }


    /// Starts threads listening to new connections.
    pub fn start(&mut self) {
        let (tcp_keepalive, timeout, addresses, tcp_backlog) = {
            let db = self.db.lock().unwrap();
            (db.config.tcp_keepalive.clone(),
            db.config.timeout.clone(),
            db.config.addresses().clone(),
            db.config.tcp_backlog.clone(),
            )
        };
        for (host, port) in addresses {
            match self.handle_tcp((&host[..], port), tcp_keepalive, timeout, tcp_backlog) {
                Ok(_) => {
                    //                    let db = self.db.lock().unwrap();
                    info!("The server is now ready to accept connections on port {}", port);
                },
                Err(err) => {
                    //                    let db = self.db.lock().unwrap();
                    warn!("Creating Server TCP listening socket {}:{}: {:?}", host, port, err);
                    continue;
                }
            }
        }

        self.handle_unixsocket();
    }

    /// Sends a kill signal to the listeners and connects to the incoming
    /// connections to break the listening loop.
    pub fn stop(&mut self) {
        #![allow(unused_must_use)]
        for sender in self.listener_channels.iter() {
            sender.send(0);
            let db = self.db.lock().unwrap();
            for (host, port) in db.config.addresses() {
                for addrs in (&host[..], port).to_socket_addrs().unwrap() {
                    TcpStream::connect(addrs);
                }
            }
        }
        self.join();
    }

    /// Join the listener threads.
    pub fn join(&mut self) {
        #![allow(unused_must_use)]
        while self.listener_threads.len() > 0 {
            self.listener_threads.pop().unwrap().join();
        }
    }
}
