use wayland_client::protocol::WaylandClient;

// use std::time::Duration;
// use std::{env, thread};
// notes: [1, 0XFEFFFFFF]
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    utils::init_log();

    let mut client = WaylandClient::connect(&utils::wayland_sockpath())?;

    client.load_interfaces()?;
    // let display_id = client.get_global(
    //     ObjectType::Display
    // ).expect("Expected to find display id");
    //
    // let registry_id = client.new_id(ObjectType::Registry);
    // info!("registry_id = {registry_id} ; display_id = {display_id}");
    // client.send_message(WaylandMessage::new_request (
    //     display_id,
    //     Request::DisplayGetRegistry(registry_id)
    // ))?;
    //
    // let events = client.fetch_events()?;

    Ok(())
}

mod utils {
    use std::env;
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
}
