use dbus::network::{get_devices, get_connections};

mod dbus;

fn main() {
    // let server = dbus::ReSetDaemon::create();
    // server.run();
     // get_devices();
    get_connections();
}
