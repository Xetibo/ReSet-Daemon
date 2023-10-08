use super::utils::call_system_dbus_method;

pub fn get_networks() {
    let res = call_system_dbus_method::<(), (Vec<String>,)>(
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "GetAllDevices",
        (),
    );
    let result = res.join();
    let result = result.unwrap().unwrap();
    for label in result.0 {
        println!("{}", label);
    }
}
