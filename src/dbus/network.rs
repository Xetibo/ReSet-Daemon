use std::time::Duration;

use dbus::{blocking::Connection, Path};

use super::utils::{call_system_dbus_method, get_system_dbus_property};

#[derive(PartialEq, Eq)]
pub enum DeviceType {
    UNKNOWN,
    GENERIC = 1,
    WIFI = 2,
    BT = 5,
    DUMMY = 22,
    OTHER,
}

impl DeviceType {
    fn from_u32(num: u32) -> Self {
        match num {
            0 => DeviceType::UNKNOWN,
            1 => DeviceType::GENERIC,
            2 => DeviceType::WIFI,
            5 => DeviceType::BT,
            22 => DeviceType::DUMMY,
            _ => DeviceType::OTHER,
        }
    }
    fn to_u32(&self) -> u32 {
        match self {
            DeviceType::UNKNOWN => 0,
            DeviceType::GENERIC => 1,
            DeviceType::WIFI => 2,
            DeviceType::BT => 5,
            DeviceType::DUMMY => 22,
            DeviceType::OTHER => 90,
        }
    }
}

#[derive(Debug)]
pub struct AccessPoint {
    ssid: Vec<u8>,
    strength: u8,
    new: bool,
    dbus_path: Path<'static>,
}

pub fn get_wifi_devices() -> Vec<Path<'static>> {
    let res = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
        "org.freedesktop.NetworkManager".to_string(),
        "/org/freedesktop/NetworkManager".to_string(),
        "GetAllDevices".to_string(),
        "org.freedesktop.NetworkManager".to_string(),
        (),
    );
    let result = res.join();
    let (result,) = result.unwrap().unwrap();
    let mut devices = Vec::new();
    for path in result {
        let device_type = get_device_type(path.to_string());
        if device_type == DeviceType::WIFI {
            devices.push(path);
        }
    }
    devices
}

pub fn get_device_type(path: String) -> DeviceType {
    let res = get_system_dbus_property::<(String, String), u32>(
        "org.freedesktop.NetworkManager".to_string(),
        path.clone(),
        "org.freedesktop.NetworkManager.Device".to_string(),
        "DeviceType".to_string(),
    );
    let result = res.join();
    let result = result.unwrap().unwrap();
    DeviceType::from_u32(result)
}

pub fn get_access_points(path: String) -> Vec<AccessPoint> {
    let res = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
        "org.freedesktop.NetworkManager".to_string(),
        path,
        "GetAllAccessPoints".to_string(),
        "org.freedesktop.NetworkManager.Device.Wireless".to_string(),
        (),
    );
    let result = res.join();
    let (result,) = result.unwrap().unwrap();
    let mut access_points = Vec::new();
    for label in result {
        access_points.push(get_access_point_properties(label));
    }
    access_points
}

pub fn get_connections() -> Vec<Path<'static>> {
    let res = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
        "org.freedesktop.NetworkManager".to_string(),
        "/org/freedesktop/NetworkManager/Settings".to_string(),
        "ListConnections".to_string(),
        "org.freedesktop.NetworkManager.Settings".to_string(),
        (),
    );
    let result = res.join();
    let (result,) = result.unwrap().unwrap();
    result
}

pub fn get_active_access_point(path: String) -> AccessPoint {
    let res = call_system_dbus_method::<(), (Path<'static>,)>(
        "org.freedesktop.NetworkManager".to_string(),
        path,
        "ActiveAccessPoint".to_string(),
        "org.freedesktop.NetworkManager.Device.Wireless".to_string(),
        (),
    );
    let result = res.join();
    let (result,) = result.unwrap().unwrap();
    get_access_point_properties(result)
}

pub fn get_access_point_properties(path: Path<'static>) -> AccessPoint {
    let interface = "org.freedesktop.NetworkManager.AccessPoint";
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(
        "org.freedesktop.NetworkManager",
        path.to_string(),
        Duration::from_millis(1000),
    );
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
    let ssid: Vec<u8> = proxy.get(interface, "Ssid").unwrap();
    let strength: u8 = proxy.get(interface, "Strength").unwrap();
    let last_seen: i32 = proxy.get(interface, "LastSeen").unwrap();
    let new;
    if last_seen == -1 {
        new = false;
    } else {
        new = true;
    }
    AccessPoint {
        ssid,
        strength,
        new,
        dbus_path: path,
    }
}

pub fn connect_to_access_point(access_point: Path<'static>, device: Path<'static>) {
    let res =
        call_system_dbus_method::<(Path<'static>, Path<'static>, Path<'static>), (Path<'static>,)>(
            "org.freedesktop.NetworkManager".to_string(),
            "/org/freedesktop/NetworkManager".to_string(),
            "ActivateConnection".to_string(),
            "org.freedesktop.NetworkManager".to_string(),
            (Path::new("").unwrap(), device, access_point),
        );
    let result = res.join();
    let result = result.unwrap().unwrap();
}

pub fn disconnect_from_access_point(connection: Path<'static>) {
    let res = call_system_dbus_method::<(Path<'static>,), ()>(
        "org.freedesktop.NetworkManager".to_string(),
        "/org/freedesktop/NetworkManager".to_string(),
        "DeactivateConnection".to_string(),
        "org.freedesktop.NetworkManager".to_string(),
        (connection,),
    );
    let result = res.join();
    let result = result.unwrap().unwrap();
}
