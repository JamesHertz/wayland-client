#![feature(
    unix_socket_ancillary_data,
    min_specialization,
    iter_next_chunk,
    iter_advance_by
)]
// #![feature(specialization)]
pub mod client;
pub mod error;
pub mod protocol;
mod wire_format;

pub use error::{Error, Result};
pub use protocol::WaylandId;
