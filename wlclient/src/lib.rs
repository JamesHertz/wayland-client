#![feature(
    unix_socket_ancillary_data,
    min_specialization,
    iter_next_chunk,
    iter_advance_by
)]

use client::WaylandClient;
use std::env;

pub mod client;
pub mod error;
pub mod protocol;
mod wire_format;

pub use error::{Error, Result};
pub use protocol::WaylandId;

pub fn init_log() {
    if env::var_os("RUST_LOG").is_none() {
        unsafe {
            env::set_var("RUST_LOG", "info");
        }
    }
    pretty_env_logger::init();
}

pub fn connect<'a, S>() -> Result<WaylandClient<'a, S>> {
    WaylandClient::connect()
}

pub fn connect_to<'a, S>(path : &str) -> Result<WaylandClient<'a, S>> {
    WaylandClient::connect_to(path)
}
