#![feature(unix_socket_ancillary_data)]
#![allow(dead_code, unused_imports)]
use std::{
    env,
    io::{IoSlice, IoSliceMut},
    os::{
        fd::{AsFd, AsRawFd},
        unix::net::{AncillaryData, SocketAncillary, UnixListener, UnixStream},
    },
    thread,
    time::Duration,
};

use log::info;
use memmap::MmapOptions;
use utils::join_shared_memory;
use wayland_client::protocol::{
    api::{WaylandObject, WaylandRequest},
    WaylandClient,
};

const SOCK_FILE: &str = "test.sock";
// notes: [1, 0XFEFFFFFF]
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    utils::init_log();

    // info!("Let start the joke");
    //
    // let mut args = env::args();
    // args.next();
    // let role = args.next().expect("Expected process role");
    //
    // match role.as_str() {
    //     "server" => {
    //         info!("Starting server ...");
    //         let sock = UnixListener::bind(SOCK_FILE)?;
    //         loop {
    //             let (connection, addr) = sock.accept()?;
    //             info!("Incomming connection from {addr:?}");
    //
    //             let mut ancillary_buffer = [0; 256];
    //             let mut ancillary = SocketAncillary::new(&mut ancillary_buffer[..]);
    //
    //             let mut buf = [1; 256];
    //             // let mut bufs = &mut [IoSliceMut::new(&mut buf[..])][..];
    //             let size = connection.recv_vectored_with_ancillary(
    //                 &mut [IoSliceMut::new(&mut buf[..])][..],
    //                 &mut ancillary,
    //             )?;
    //
    //             let fds: Vec<i32> = ancillary
    //                 .messages()
    //                 .flat_map(|msg| {
    //                     if let AncillaryData::ScmRights(scm_rights) = msg.unwrap() {
    //                         scm_rights.into_iter().collect()
    //                     } else {
    //                         vec![]
    //                     }
    //                 })
    //                 .collect();
    //             info!("received {size} bytes and fds {fds:?}");
    //
    //             assert!(!fds.is_empty());
    //             let original = i32::from_ne_bytes(
    //                 buf[..4].try_into().expect("Getting original fd")
    //             );
    //
    //             let mapped_size = usize::from_ne_bytes (
    //                 buf[4..size].try_into().expect("Getting mapped size")
    //             );
    //
    //             info!("Original fd was {original} and mapped size {mapped_size}");
    //
    //             let mem = join_shared_memory(fds[0], mapped_size)?;
    //             info!("Created mappend memory");
    //
    //             let buffer = mem.as_ref();
    //             for _ in 0..1000 {
    //                 info!("buffer position 0 is {}", buffer[0]);
    //                 thread::sleep(Duration::from_secs(10));
    //             }
    //
    //             info!("Done, waiting for a new client")
    //         }
    //     }
    //     "client" => {
    //         info!("starting the client ...");
    //         let mapped_size = 1024;
    //         let file = utils::shared_memory(mapped_size)?;
    //         let fd = file.as_raw_fd();
    //         info!("Sucessfully created shared memory with fd {fd}");
    //
    //         let sock = UnixStream::connect(SOCK_FILE)?;
    //
    //         let mut ancillary_buffer = [0; 128];
    //         let mut ancillary = SocketAncillary::new(&mut ancillary_buffer[..]);
    //         ancillary.add_fds(&[fd][..]);
    //
    //         let mut buf = [0; 256];
    //         buf[..4].copy_from_slice(
    //             &fd.to_ne_bytes()
    //         );
    //         buf[4..12].copy_from_slice(
    //             &mapped_size.to_ne_bytes()
    //         );
    //
    //         // let buf = fd.to_ne_bytes();
    //         let size =
    //             sock.send_vectored_with_ancillary(&[IoSlice::new(&buf[..12])][..], &mut ancillary)?;
    //
    //         info!("Sent {size} bytes");
    //
    //         thread::sleep(Duration::from_secs(5));
    //         let mut mem = unsafe { MmapOptions::new().map_mut(&file)? };
    //         let buffer = mem.as_mut();
    //         let mut i = 0u8;
    //         loop {
    //             buffer[0] = i;
    //             info!("Settted buf 0 to {i}");
    //             i = (i + 1) % u8::MAX;
    //             thread::sleep(Duration::from_secs(10));
    //         }
    //
    //         // loop{}
    //     }
    //     _ => panic!("Invalid role {role}"),
    // }

    let mut client = WaylandClient::connect(&utils::wayland_sockpath())?;

    client.load_interfaces()?;

    info!("Waiting to see if any error will arrive!");
    thread::sleep(Duration::from_secs(30));

    let messages = client.pull_messages()?;
    info!("Gotten messages {messages:?}");
    // let compositor   = client.get_global(WaylandObject::Compositor).unwrap();
    // let sufurface_id = client.new_id(WaylandObject::Surface);
    // client.send_request(
    //     compositor, WaylandRequest::CompositorCreateSurface(sufurface_id)
    // )?;
    //
    // let wm = client.get_global(WaylandObject::XdgWmBase).unwrap();
    // let xdg_surface_id = client.new_id(WaylandObject::XdgSurface);
    // client.send_request(
    //     wm, WaylandRequest::XdgWmGetSurface { new_id: xdg_surface_id, surface: sufurface_id }
    // )?;
    //
    // let messages = client.pull_msgs();
    // info!("You've got {:?} messages", messages);
    Ok(())
}

mod utils {

    use color_eyre::eyre::eyre;
    use log::info;
    use memmap::{MmapMut, MmapOptions};
    use std::{env, fs::File, os::fd::FromRawFd, thread, time::Duration};

    pub fn wayland_sockpath() -> String {
        format!(
            "{}/{}",
            env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR var "),
            env::var("WAYLAND_DISPLAY").expect("WAYLAND_DISPLAY var "),
        )
    }

    pub fn init_log() {
        if env::var_os("RUST_LOG").is_none() {
            unsafe {
                env::set_var("RUST_LOG", "info");
            }
        }
        pretty_env_logger::init();
    }

    pub fn join_shared_memory(
        fd: i32,
        size: usize,
    ) -> color_eyre::Result<MmapMut> {
        info!("fd = {}", fd);
        let file = unsafe { File::from_raw_fd(fd) };

        file.set_len(size as u64)?;

        let map = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(map)
    }

    pub fn shared_memory(size: usize) -> color_eyre::Result<File> {
        let filename = b"my-custom-file\0".as_ptr() as *const libc::c_char;
        let fd = unsafe {
            libc::shm_open(
                filename,
                libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                0o666,
            )
        };

        if fd < 0 {
            return Err(eyre!(
                "Error creating with shm_open '{}'",
                errno::errno()
            ));
        }

        let res = unsafe { libc::shm_unlink(filename) };
        if res < 0 {
            return Err(eyre!("Error unlinking '{}'", errno::errno()));
        }

        let file = unsafe { File::from_raw_fd(fd) };

        file.set_len(size as u64)?;

        // let map = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(file)
    }
}
