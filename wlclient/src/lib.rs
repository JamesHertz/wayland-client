#![feature(
    unix_socket_ancillary_data,
    min_specialization,
    iter_next_chunk,
    iter_advance_by
)]

pub mod client;
pub mod error;
pub mod protocol;

pub use error::{Error, Result};
pub use client::WaylandClient;

use std::env;

pub fn init_log() {
    if env::var_os("RUST_LOG").is_none() {
        unsafe {
            env::set_var("RUST_LOG", "info");
        }
    }
    pretty_env_logger::init();
}

pub fn connect<S>() -> Result<WaylandClient<S>> {
    WaylandClient::connect()
}

pub fn connect_to<S>(path : &str) -> Result<WaylandClient<S>> {
    WaylandClient::connect_to(path)
}
