
use std::env;
use std::fs;
use std::io;
use std::io::{Read, Write, Seek};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nix::fcntl::{flock, FlockArg};


use blake2_rfc::blake2b::{Blake2b};
use uuid::Uuid;


use ::types::ContentId;

use super::config::Config;


pub type DatabaseHolder = Arc<Mutex<Database>>;


#[derive(Debug)]
pub struct Database {
    pub config: Config,
    pub version: &'static str,
    pub rustc_version: &'static str,
    pub run_id: Uuid,
}




// --------------------------------------------------------------------------------------------------------------------


fn check_dir(dir: &Path) -> io::Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir)?
    }
    Ok(())
}


/// Computes the checksum of a file. File must be open for reading
fn checksum_file(f: &mut fs::File) -> io::Result<Vec<u8>> {
    const BUF_SIZE: usize = 4096;
    f.seek(io::SeekFrom::Start(0))?;
    let mut buf = [0u8; BUF_SIZE];
    let mut context = Blake2b::new(64);
    context.update(&buf[0..0]);
    'read_file: loop {
        let len = f.read(&mut buf)?;
        if len == 0 { break 'read_file; };
        context.update(&buf[0..len]);
    }
    Ok(context.finalize().as_bytes().to_vec())
}

// --------------------------------------------------------------------------------------------------------------------


const READ_BUFFER_SIZE: usize = 4096;


impl Database {
    pub fn new(config: Config) -> io::Result<Self> {

        check_dir(config.workdir)?;
        check_dir(config.filesdir)?;
        env::set_current_dir(config.workdir)?;

        let db = Database {
            config: config,
            version: "0.0.0",
            rustc_version: "",
            run_id: Uuid::new_v4(),
        };

        Ok(db)
    }

    pub fn make_path(&self, hash: &str) -> (PathBuf, PathBuf) {
        assert!(hash.len() > 4);
        let dir_path = self.config.filesdir.join(&hash[0..2]).join(&hash[2..4]);
        let file_path = dir_path.join(&hash[4..]);
        (dir_path, file_path)
    }

    pub fn copy_from(db: DatabaseHolder, uri: &str) -> io::Result<ContentId> {
        let mut options = fs::OpenOptions::new();

        let path = Path::new(uri);
        let mut input = options.read(true).append(false).open(path).unwrap();
        // TODO lock timeout
        flock(input.as_raw_fd(), FlockArg::LockExclusive).unwrap();

        let tmp_name = format!("{}", Uuid::new_v4().hyphenated());
        let tmp_path = Path::new(".").canonicalize()?.join(tmp_name).with_extension("tmp");
        let mut output = options.create(true).append(true).open(&tmp_path).unwrap();
        flock(output.as_raw_fd(), FlockArg::LockExclusive).unwrap();

        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut context = Blake2b::new(64);
        context.update(&buf[0..0]);

        'read_file: loop {
            let len = input.read(&mut buf).unwrap();
            if len == 0 { break 'read_file; };
            context.update(&buf[0..len]);
            output.write(&buf[0..len]).unwrap();
        }

        let hash = ContentId::from_slice(context.finalize().as_bytes());
        let hash_name = hash.to_string();

        let (dir_path, file_path) = db.lock().unwrap().make_path(&hash_name);
        check_dir(&dir_path).unwrap();

        fs::rename(tmp_path, file_path).unwrap();
        Ok(hash)
    }
}


#[cfg(test)]
mod tests {
    #![cfg_attr(feature = "clippy", allow(result_unwrap_used))]

    use std::path::Path;
    use std::io::{Read, Write, Seek};
    use std::io;
    use std::fs;
    use std::os::unix::io::AsRawFd;

    use blake2_rfc::blake2b::{blake2b};
    use nix::fcntl::{flock, FlockArg};

    use super::checksum_file;


    fn path<'a>() -> &'a Path {
        Path::new("/tmp/fs-test.tmp")
    }

    fn create_tmp() -> fs::File {
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(true);
        let file = options.open(path()).unwrap();
        flock(file.as_raw_fd(), FlockArg::LockExclusive).unwrap();
        file.set_len(0);
        file
    }

    #[test]
    fn empty() {
        let mut tmp = create_tmp();
        let a = checksum_file(&mut tmp).unwrap();
        let b = blake2b(64, &[], b"");
        assert_eq!(a.as_slice(), b.as_bytes());
    }

    #[test]
    fn z123() {
        let mut tmp = create_tmp();
        let sample = b"123";
        tmp.write_all(sample).unwrap();

        let a = checksum_file(&mut tmp).unwrap();
        let b = blake2b(64, &[], sample);

        assert_eq!(a.as_slice(), b.as_bytes());
    }
}
