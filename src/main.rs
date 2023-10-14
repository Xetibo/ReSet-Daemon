use dbus::reset_dbus::Daemon;

mod dbus;

fn main() {
    // example disconnect
    // get wifi device
    // let mut devices = get_wifi_devices();
    // let mut device = devices.pop().unwrap();
    // dbg!(get_stored_connections());
    // dbg!(device.connect_to_access_point("gurrigear"));
    // disconnect from current
    // device.disconnect_from_current().unwrap();
    let daemon = Daemon::create();
    if daemon.is_err() {
        return;
    }
    let mut daemon = daemon.unwrap();
    daemon.run();
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
