use std::path::Path;


#[derive(Debug)]
pub struct Config {
    pub pidfile: &'static Path,
    pub workdir: &'static Path,
    pub filesdir: &'static Path,

    pub daemonize: bool,
    pub hz: u32,

    pub bind: Vec<String>,
    pub port: u16,
    pub tcp_keepalive: u32,
    pub tcp_backlog: i32,
    pub timeout: u64,

    pub unixsocket: Option<String>,
    pub unixsocketperm: u32,

    pub syslog_enabled: bool,
    pub syslog_ident: String,
    pub syslog_facility: String,
}

//#[derive(Debug)]
//pub enum ConfigError {
//    InvalidFormat,
//    InvalidParameter,
//    IOError(IOError),
//    FileNotFound,
//}


impl Config {
    pub fn default(port: u16) -> Config {

        Config {
            pidfile: Path::new("/var/run/irbis/ifsd.pid"),
            workdir: Path::new("/srv/irbisfs/content"),
            filesdir: Path::new("/srv/irbisfs/content/files"),

            daemonize: false,
            hz: 10,

            // FIXME список адресов в конфигурации должен быть уже в валидном виде
            bind: vec![],
            // FIXME порт должен соответствовать каждому интерфейсу
            port: port,
            tcp_keepalive: 0,
            tcp_backlog: 511,
            timeout: 0,

            unixsocket: None,
            unixsocketperm: 0700,

            syslog_enabled: false,
            syslog_ident: "ifsd".to_owned(),
            syslog_facility: "local0".to_owned(),
        }
    }

    pub fn new() -> Config {
        Self::default(1313)
    }

    // FIXME никаких заглушек "на случай если нет", список адресов должен формироваться сразу в конфигурации
    pub fn addresses(&self) -> Vec<(String, u16)> {
        if self.bind.len() == 0 {
            vec![("127.0.0.1".to_owned(), self.port)]
        } else {
            self.bind.iter().map(|s| (s.clone(), self.port)).collect::<Vec<_>>()
        }
    }
}
