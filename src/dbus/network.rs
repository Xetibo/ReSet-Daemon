use dbus::Path;

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

pub fn get_devices() {
    let res = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
        "org.freedesktop.NetworkManager".to_string(),
        "/org/freedesktop/NetworkManager".to_string(),
        "GetAllDevices".to_string(),
        "org.freedesktop.NetworkManager".to_string(),
        (),
    );
    let result = res.join();
    let (result,) = result.unwrap().unwrap();
    for path in result {
        let device_type = get_device_type(path.to_string());
        if device_type == DeviceType::WIFI {
            println!("{} and {}", device_type.to_u32(), path.to_string());
            get_networks(path.to_string());
        }
    }
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

pub fn get_networks(path: String) {
    let res = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
        "org.freedesktop.NetworkManager".to_string(),
        path,
        "GetAllAccessPoints".to_string(),
        "org.freedesktop.NetworkManager.Device.Wireless".to_string(),
        (),
    );
    let result = res.join();
    let result = result.unwrap().unwrap();
    for label in result.0 {
        println!("{}", label);
    }
}

pub fn get_access_point_properties() {}
pub fn connect_to_access_point() {}
pub fn disconnect_from_access_point() {}
