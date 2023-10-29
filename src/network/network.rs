use core::fmt;
use std::{
    collections::HashMap,
    str,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};

use dbus::{
    arg::{self, Append, Arg, ArgType, Get, PropMap, RefArg, Variant},
    blocking::Connection,
    message::SignalArgs,
    Path, Signature,
};
use dbus_crossroads::Context;

use crate::utils::{call_system_dbus_method, get_system_dbus_property};

use super::network_signals::{AccessPointAdded, AccessPointRemoved};

#[derive(Debug, Clone)]
pub struct Error {
    pub message: &'static str,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionError {
    method: &'static str,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not {} Access Point.", self.method)
    }
}

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

#[derive(Debug, Clone)]
pub struct AccessPoint {
    pub ssid: Vec<u8>,
    pub strength: u8,
    pub associated_connection: Path<'static>,
    pub dbus_path: Path<'static>,
}

impl Append for AccessPoint {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            let sig = unsafe { Signature::from_slice_unchecked("y\0") };
            i.append_array(&sig, |i| {
                for byte in self.ssid.iter() {
                    i.append(byte);
                }
            });
            i.append(&self.strength);
            i.append(&self.associated_connection);
            i.append(&self.dbus_path);
        });
    }
}

impl<'a> Get<'a> for AccessPoint {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (ssid, strength, associated_connection, dbus_path) =
            <(Vec<u8>, u8, Path<'static>, Path<'static>)>::get(i)?;
        Some(AccessPoint {
            ssid,
            strength,
            associated_connection,
            dbus_path,
        })
    }
}

impl Arg for AccessPoint {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ayyoo)\0") }
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    pub access_point: Option<AccessPoint>,
    pub connection: Option<Path<'static>>,
    pub dbus_path: Path<'static>,
    pub connected: bool,
}

impl Device {
    pub fn from_path(path: Path<'static>) -> Self {
        Self {
            access_point: None,
            connection: None,
            dbus_path: path,
            connected: false,
        }
    }

    pub fn start_listener(&self, ctx: Arc<Mutex<Context>>) -> Result<(), dbus::Error> {
        let conn = Connection::new_system().unwrap();
        let mr = AccessPointAdded::match_rule(
            Some(&"org.freedesktop.NetworkManager".into()),
            Some(&self.dbus_path),
        )
        .static_clone();
        let mrb = AccessPointRemoved::match_rule(
            Some(&"org.freedesktop.NetworkManager".into()),
            Some(&self.dbus_path),
        )
        .static_clone();
        let ctx_ref = ctx.clone();
        let res = conn.add_match(mr, move |ir: AccessPointAdded, _, _| {
            println!("access point added");
            let mut context = ctx_ref.lock().unwrap();
            let signal = context.make_signal(
                "AccessPointAdded",
                (get_access_point_properties(ir.access_point),),
            );
            context.push_msg(signal);
            true
        });
        if res.is_err() {
            return Err(dbus::Error::new_custom(
                "SignalMatchFailed",
                "Failed to match signal on NetworkManager.",
            ));
        }
        let res = conn.add_match(mrb, move |ir: AccessPointRemoved, _, _| {
            println!("access point removed");
            let mut context = ctx.lock().unwrap();
            let signal = context.make_signal("AccessPointRemoved", (ir.access_point,));
            context.push_msg(signal);
            true
        });
        if res.is_err() {
            return Err(dbus::Error::new_custom(
                "SignalMatchFailed",
                "Failed to match signal on NetworkManager.",
            ));
        }

        let now = SystemTime::now();
        loop {
            let _ = conn.process(Duration::from_millis(1000))?;
            if now.elapsed().unwrap() > Duration::from_millis(60000) {
                break;
            }
        }
        Ok(())
    }
}

pub fn get_wifi_devices() -> Vec<Device> {
    let result = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
        "org.freedesktop.NetworkManager",
        Path::from("/org/freedesktop/NetworkManager"),
        "GetAllDevices",
        "org.freedesktop.NetworkManager",
        (),
        1000,
    );
    let (result,) = result.unwrap();
    let mut devices = Vec::new();
    for path in result {
        let device_type = get_device_type(path.to_string());
        if device_type == DeviceType::WIFI {
            let mut device = Device::from_path(path);
            device.initialize();
            devices.push(device);
        }
    }
    devices
}

pub fn get_device_type(path: String) -> DeviceType {
    let result = get_system_dbus_property::<(String, String), u32>(
        "org.freedesktop.NetworkManager",
        Path::from(path),
        "org.freedesktop.NetworkManager.Device",
        "DeviceType",
    );
    let result = result.unwrap();
    DeviceType::from_u32(result)
}

pub fn get_connection_settings(path: Path<'static>) -> HashMap<String, PropMap> {
    let result = call_system_dbus_method::<(), (HashMap<String, PropMap>,)>(
        "org.freedesktop.NetworkManager",
        path,
        "GetSettings",
        "org.freedesktop.NetworkManager.Settings.Connection",
        (),
        1000,
    );
    let (result,): (HashMap<String, PropMap>,) = result.unwrap();
    result
}

pub fn set_password(path: Path<'static>, password: String) {
    // yes this will be encrypted later
    let password = Box::new(password) as Box<dyn RefArg>;
    let mut settings = get_connection_settings(path.clone());
    settings
        .get_mut("802-11-wireless-security")
        .unwrap()
        .insert("password".to_string(), Variant(password));
    let result = call_system_dbus_method::<(HashMap<String, PropMap>,), ()>(
        "org.freedesktop.NetworkManager",
        path,
        "Update",
        "org.freedesktop.NetworkManager.Settings.Connection",
        (settings,),
        1000,
    );
    result.unwrap();
}

pub fn get_connection_secrets(path: Path<'static>) {
    let result = call_system_dbus_method::<(String,), (HashMap<String, PropMap>,)>(
        "org.freedesktop.NetworkManager",
        path,
        "GetSecrets",
        "org.freedesktop.NetworkManager.Settings.Connection",
        ("802-11-wireless-security".to_string(),),
        1000,
    );
    let (_,): (HashMap<String, PropMap>,) = result.unwrap();
    // result
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
    let mut associated_connection: Option<Path<'static>> = None;
    let connections = get_stored_connections();
    for (connection, connection_ssid) in connections {
        if ssid == connection_ssid {
            associated_connection = Some(connection);
        }
    }
    if associated_connection.is_none() {
        associated_connection = Some(Path::from("/"));
    }
    AccessPoint {
        ssid,
        strength,
        associated_connection: associated_connection.unwrap(),
        dbus_path: path,
    }
}

pub fn get_active_connections() -> Vec<Path<'static>> {
    let interface = "org.freedesktop.NetworkManager";
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager".to_string(),
        Duration::from_millis(1000),
    );
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
    let connections: Vec<Path<'static>> = proxy.get(interface, "ActiveConnections").unwrap();
    connections
}

pub fn get_associations_of_active_connection(
    path: Path<'static>,
) -> (Vec<Path<'static>>, Option<AccessPoint>) {
    let interface = "org.freedesktop.NetworkManager.Connection.Active";
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(
        "org.freedesktop.NetworkManager",
        path,
        Duration::from_millis(1000),
    );
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
    let devices: Vec<Path<'static>> = proxy.get(interface, "Devices").unwrap();
    let access_point_prop: Path<'static> = proxy.get(interface, "SpecificObject").unwrap();
    let connection_type: String = proxy.get(interface, "Type").unwrap();
    let access_point: Option<AccessPoint>;
    if connection_type == "802-11-wireless" {
        access_point = Some(get_access_point_properties(access_point_prop));
    } else {
        access_point = None;
    }
    (devices, access_point)
}

pub fn get_stored_connections() -> Vec<(Path<'static>, Vec<u8>)> {
    let result = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
        "org.freedesktop.NetworkManager",
        Path::from("/org/freedesktop/NetworkManager/Settings"),
        "ListConnections",
        "org.freedesktop.NetworkManager.Settings",
        (),
        1000,
    );
    let (result,) = result.unwrap();
    let mut wifi_connections = Vec::new();
    for connection in result {
        let settings = get_connection_settings(connection.clone());
        let settings = settings.get("802-11-wireless");
        if settings.is_some() {
            let settings = settings.unwrap();
            let ssid: &Vec<u8> = arg::prop_cast(settings, "ssid").unwrap();
            let ssid = ssid.clone();
            wifi_connections.push((connection, ssid));
        }
    }
    wifi_connections
}

pub fn disconnect_from_access_point(connection: Path<'static>) -> Result<(), ConnectionError> {
    let result = call_system_dbus_method::<(Path<'static>,), ()>(
        "org.freedesktop.NetworkManager",
        Path::from("/org/freedesktop/NetworkManager"),
        "DeactivateConnection",
        "org.freedesktop.NetworkManager",
        (connection,),
        1000,
    );
    if result.is_err() {
        return Err(ConnectionError {
            method: "disconnect from",
        });
    }
    Ok(())
}

impl Device {
    pub fn initialize(&mut self) {
        let connections = get_active_connections();
        for connection in connections {
            let (devices, access_point) = get_associations_of_active_connection(connection.clone());
            if devices.contains(&self.dbus_path) {
                self.connection = Some(connection);
                self.access_point = access_point;
                self.connected = true;
            }
        }
    }

    pub fn get_access_points(&self) -> Vec<AccessPoint> {
        let result = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
            "org.freedesktop.NetworkManager",
            self.dbus_path.clone(),
            "GetAllAccessPoints",
            "org.freedesktop.NetworkManager.Device.Wireless",
            (),
            1000,
        );
        let (result,) = result.unwrap();
        let mut access_points = Vec::new();
        for label in result {
            access_points.push(get_access_point_properties(label));
        }
        access_points
    }

    pub fn set_active_access_point(&mut self) {
        let interface = "org.freedesktop.NetworkManager.Device.Wireless";
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy(
            "org.freedesktop.NetworkManager",
            self.dbus_path.clone(),
            Duration::from_millis(1000),
        );
        use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
        let access_point: Path<'static> = proxy.get(interface, "ActiveAccessPoint").unwrap();
        self.access_point = Some(get_access_point_properties(access_point))
    }

    pub fn connect_to_access_point(
        &mut self,
        access_point: AccessPoint,
    ) -> Result<(), ConnectionError> {
        let result = call_system_dbus_method::<
            (Path<'static>, Path<'static>, Path<'static>),
            (Path<'static>,),
        >(
            "org.freedesktop.NetworkManager",
            Path::from("/org/freedesktop/NetworkManager"),
            "ActivateConnection",
            "org.freedesktop.NetworkManager",
            (
                access_point.associated_connection,
                self.dbus_path.clone(),
                Path::from("/"),
            ),
            1000,
        );
        if result.is_err() {
            return Err(ConnectionError {
                method: "connect to",
            });
        }
        let (result,) = result.unwrap();
        let connection = get_associations_of_active_connection(result.clone());
        self.connection = Some(result);
        self.access_point = connection.1;
        self.connected = true;
        Ok(())
    }

    pub fn add_and_connect_to_access_point(
        &mut self,
        access_point: AccessPoint,
        password: String,
    ) -> Result<(), ConnectionError> {
        let mut properties = HashMap::new();
        properties.insert("802-11-wireless-security".to_string(), PropMap::new());
        let password = Box::new(password) as Box<dyn RefArg>;
        properties
            .get_mut("802-11-wireless-security")
            .unwrap()
            .insert("psk".to_string(), Variant(password));
        let result = call_system_dbus_method::<
            (HashMap<String, PropMap>, Path<'static>, Path<'static>),
            (Path<'static>, Path<'static>),
        >(
            "org.freedesktop.NetworkManager",
            Path::from("/org/freedesktop/NetworkManager"),
            "AddAndActivateConnection",
            "org.freedesktop.NetworkManager",
            (
                properties,
                self.dbus_path.clone(),
                access_point.dbus_path.clone(),
            ),
            1000,
        );
        if result.is_ok() {
            let result = result.unwrap();
            (self.connection, self.access_point) = (
                Some(result.1),
                Some(get_access_point_properties(access_point.dbus_path)),
            );
            return Ok(());
        }
        Err(ConnectionError {
            method: "connect to",
        })
    }

    pub fn disconnect_from_current(&mut self) -> Result<(), ConnectionError> {
        if self.connected {
            let res = disconnect_from_access_point(self.connection.clone().unwrap());
            if res.is_err() {
                return Err(ConnectionError {
                    method: "disconnect from",
                });
            }
            self.connected = false;
            self.access_point = None;
            self.connection = None;
        }
        Ok(())
    }
}