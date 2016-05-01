use std::net::SocketAddr;


#[derive(Clone, Debug)]
pub enum Target {
    Unix(String),
    Tcp(SocketAddr),
}


pub struct Config {
    pub tcp_keepalive: u32,
    pub tcp_backlog: i32,
    pub timeout: u64,
    pub target: Target,
}

//#[derive(Debug)]
//pub enum ConfigError {
//    InvalidFormat,
//    InvalidParameter,
//    IOError(IOError),
//    FileNotFound,
//}


// --------------------------------------------------------------------------------------------------------------------


impl Config {
    pub fn default(target: Target) -> Config {
        Config {
            tcp_keepalive: 0,
            tcp_backlog: 511,
            timeout: 0,
            target: target
        }
    }

    pub fn new(target: Target) -> Config {
        Self::default(target)
    }
}
