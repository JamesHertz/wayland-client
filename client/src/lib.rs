#![feature(unix_socket_ancillary_data, min_specialization)]
// #![feature(specialization)]
pub mod protocol;
pub mod error;
pub mod client;
// mod wire_message;

pub use error::{Result, Error};
pub use protocol::{WaylandId};
