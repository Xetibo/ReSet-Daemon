use std::{rc::Rc, cell::RefCell, ops::Deref};

use dbus::{bluetooth::BluetoothInterface, reset_dbus::Daemon, audio::PulseServer};
use pulse::{self, mainloop::standard::{Mainloop, IterateResult}, context::{Context, FlagSet, introspect::SourceInfo}, proplist::Proplist, callbacks::ListResult};


mod dbus;

#[tokio::main]
pub async fn main() {
    // let daemon = Daemon::create();
    // if daemon.is_err() {
    //     return;
    // }
    // let mut daemon = daemon.unwrap();
    // daemon.run();
    // let bl = BluetoothInterface::create();
    // if bl.is_some() {
        // let mut bl = bl.unwrap();
        // bl.get_connections();
        // dbg!(bl);
        // bl.set_bluetooth(false);
    // }
    let daemon = Daemon::create();
    if daemon.is_err() {
        return;
    }
    let mut daemon = daemon.unwrap();
    daemon.run().await;
    //     let mut bl = bl.unwrap();
    //     bl.get_connections();
    //     dbg!(bl);
    //     // bl.set_bluetooth(false);
    // }
     // let daemon = Daemon::create();
     // if daemon.is_err() {
     //     return;
     // }
     // let mut daemon = daemon.unwrap();
     // daemon.run();
}

// // example disconnect
// // get wifi device
// let mut devices = get_wifi_devices();
// let mut device = devices.pop().unwrap();
// // disconnect from current
// device.disconnect_from_current().unwrap();

// // example connect to new access point
// // get wifi device
// let mut devices = get_wifi_devices();
// let mut device = devices.pop().unwrap();
// let access_points = device.get_access_points();
// println!(
//     "connecting to {}",
//     String::from_utf8(access_points.get(0).unwrap().ssid.clone()).unwrap()
// );
// let res = device.add_and_connect_to_access_point(
//     access_points.get(0).unwrap().dbus_path.clone(),
//     "password,".to_string(),
// );
