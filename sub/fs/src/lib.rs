#![cfg_attr(feature = "dev", allow(unstable_features))]

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#![cfg_attr(feature = "trace", feature(custom_attribute, plugin))]
#![cfg_attr(feature = "trace", plugin(trace))]

#![feature(question_mark)]

#[cfg(all(feature = "bench", test))]
extern crate test;

extern crate blake2_rfc;
#[cfg(unix)] extern crate libc;
#[macro_use] extern crate log;
extern crate net2;
#[cfg(unix)] extern crate nix;
#[macro_use] extern crate slice_as_array;
#[cfg(unix)] extern crate unix_socket;
extern crate uuid;

extern crate compat;
extern crate protocol;


pub mod client;
pub mod connection;
pub mod proto;
pub mod server;
pub mod types;



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
