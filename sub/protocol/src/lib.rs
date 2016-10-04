#![feature(custom_attribute, plugin)]

#![cfg_attr(feature = "trace", plugin(trace))]

#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", allow(items_after_statements))]

#[macro_use] extern crate log;
extern crate net2;
#[cfg(unix)] extern crate unix_socket;


pub mod message;
pub mod serde;
pub mod stream;
pub mod workflow;
