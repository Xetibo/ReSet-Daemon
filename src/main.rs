use dbus::network::get_devices;

mod dbus;

fn main() {
    // let server = dbus::ReSetDaemon::create();
    // server.run();
     get_devices();
}
