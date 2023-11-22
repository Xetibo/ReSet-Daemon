use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    thread,
    time::{Duration, SystemTime},
};

use dbus::{
    arg::{self, PropMap, RefArg, Variant},
    blocking::Connection,
    channel::Sender,
    message::SignalArgs,
    nonblock::SyncConnection,
    Message, Path,
};
use ReSet_Lib::{
    network::{
        network::{AccessPoint, ConnectionError, DeviceType},
        network_signals::{AccessPointAdded, AccessPointRemoved},
    },
    utils::{call_system_dbus_method, get_system_dbus_property},
};

#[derive(Debug)]
pub struct AccessPointChanged {
    pub interface: String,
    pub map: PropMap,
    pub invalid: Vec<String>,
}

impl arg::AppendAll for AccessPointChanged {
    fn append(&self, i: &mut arg::IterAppend) {
        arg::RefArg::append(&self.interface, i);
        arg::RefArg::append(&self.map, i);
        arg::RefArg::append(&self.invalid, i);
    }
}

impl arg::ReadAll for AccessPointChanged {
    fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
        Ok(AccessPointChanged {
            interface: i.read()?,
            map: i.read()?,
            invalid: i.read()?,
        })
    }
}

impl dbus::message::SignalArgs for AccessPointChanged {
    const NAME: &'static str = "PropertiesChanged";
    const INTERFACE: &'static str = "org.freedesktop.DBus.Properties";
}

#[derive(Debug)]
pub struct Device {
    pub access_point: Option<AccessPoint>,
    pub connection: Option<Path<'static>>,
    pub dbus_path: Path<'static>,
    pub connected: bool,
    pub active_listener: AtomicBool,
}

impl Clone for Device {
    fn clone(&self) -> Self {
        Self {
            access_point: self.access_point.clone(),
            connection: self.connection.clone(),
            dbus_path: self.dbus_path.clone(),
            connected: self.connected,
            active_listener: AtomicBool::new(false),
        }
    }
}

impl Device {
    pub fn new(path: Path<'static>) -> Self {
        Self {
            access_point: None,
            connection: None,
            dbus_path: path,
            connected: false,
            active_listener: AtomicBool::new(false),
        }
    }
}

#[allow(unused_variables)]
pub fn start_listener(
    connection: Arc<SyncConnection>,
    device: Arc<RwLock<Device>>,
    path: Path<'static>,
    active_listener: Arc<AtomicBool>,
) -> Result<(), dbus::Error> {
    let access_point_added_ref = connection.clone();
    let access_point_removed_ref = connection.clone();
    let device_ref = device.clone();
    let conn = Connection::new_system().unwrap();
    let access_point_added =
        AccessPointAdded::match_rule(Some(&"org.freedesktop.NetworkManager".into()), Some(&path))
            .static_clone();
    let access_point_removed =
        AccessPointRemoved::match_rule(Some(&"org.freedesktop.NetworkManager".into()), Some(&path))
            .static_clone();
    let access_point_changed =
        AccessPointChanged::match_rule(Some(&"org.freedesktop.NetworkManager".into()), None)
            .static_clone();
    let res = conn.add_match(
        access_point_changed,
        move |ir: AccessPointChanged, _, msg| {
            match ir.interface.as_str() {
                "org.freedesktop.NetworkManager.AccessPoint" => {
                    let path = msg.path().unwrap().to_string();
                    if path.contains("/org/freedesktop/NetworkManager/AccessPoint/") {
                        let mut connected = false;
                        let current_access_point = device_ref.read().unwrap().access_point.clone();
                        if current_access_point.is_some() {
                            let current_path = current_access_point.unwrap().dbus_path;
                            if Path::from(path.clone()) == current_path {
                                connected = true;
                            }
                        }
                        let access_point = get_access_point_properties(connected, Path::from(path));
                        let msg = Message::signal(
                            &Path::from("/org/xetibo/ReSet"),
                            &"org.xetibo.ReSet".into(),
                            &"AccessPointChanged".into(),
                        )
                        .append1(access_point);
                        let _ = connection.send(msg);
                    }
                }
                _ => println!("other event"),
            }
            true
        },
    );
    if res.is_err() {
        return Err(dbus::Error::new_custom(
            "SignalMatchFailed",
            "Failed to match signal on NetworkManager.",
        ));
    }
    let res = conn.add_match(access_point_added, move |ir: AccessPointAdded, conn, _| {
        let msg = Message::signal(
            &Path::from("/org/xetibo/ReSet"),
            &"org.xetibo.ReSet".into(),
            &"AccessPointAdded".into(),
        )
        .append1(get_access_point_properties(false, ir.access_point));
        let _ = access_point_added_ref.send(msg);
        true
    });
    if res.is_err() {
        return Err(dbus::Error::new_custom(
            "SignalMatchFailed",
            "Failed to match signal on NetworkManager.",
        ));
    }
    let res = conn.add_match(
        access_point_removed,
        move |ir: AccessPointRemoved, conn, _| {
            let msg = Message::signal(
                &Path::from("/org/xetibo/ReSet"),
                &"org.xetibo.ReSet".into(),
                &"AccessPointRemoved".into(),
            )
            .append1(ir.access_point);
            let _ = access_point_removed_ref.send(msg);
            true
        },
    );
    if res.is_err() {
        return Err(dbus::Error::new_custom(
            "SignalMatchFailed",
            "Failed to match signal on NetworkManager.",
        ));
    }
    println!("started listener");
    active_listener.store(true, Ordering::SeqCst);
    let mut time = SystemTime::now();
    loop {
        let _ = conn.process(Duration::from_millis(1000))?;
        if !active_listener.load(Ordering::SeqCst) {
            println!("stopped listener");
            break;
        }
        if time.elapsed().unwrap_or(Duration::from_millis(0)) < Duration::from_secs(10) {
            time = SystemTime::now();
            device.read().unwrap().request_scan();
        }
        thread::sleep(Duration::from_millis(1000));
    }
    Ok(())
}

pub fn stop_listener(active_listener: Arc<AtomicBool>) {
    active_listener.store(false, Ordering::SeqCst);
}

pub fn get_wifi_devices() -> Vec<Arc<RwLock<Device>>> {
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
            let mut device = Device::new(path);
            device.initialize();
            devices.push(Arc::new(RwLock::new(device)));
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

pub fn list_connections() -> Vec<Path<'static>> {
    let result = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager/Settings".into(),
        "ListConnections",
        "org.freedesktop.NetworkManager.Settings",
        (),
        1000,
    );
    let (result,): (Vec<Path<'static>>,) = result.unwrap();
    result
}

pub fn get_connection_settings(
    path: Path<'static>,
) -> Result<
    (
        HashMap<
            std::string::String,
            HashMap<std::string::String, dbus::arg::Variant<Box<dyn RefArg>>>,
        >,
    ),
    dbus::Error,
> {
    let result = call_system_dbus_method::<(), (HashMap<String, PropMap>,)>(
        "org.freedesktop.NetworkManager",
        path,
        "GetSettings",
        "org.freedesktop.NetworkManager.Settings.Connection",
        (),
        1000,
    );
    result
}

pub fn set_connection_settings(path: Path<'static>, settings: HashMap<String, PropMap>) -> bool {
    let result = call_system_dbus_method::<(HashMap<String, PropMap>,), ()>(
        "org.freedesktop.NetworkManager",
        path,
        "Update",
        "org.freedesktop.NetworkManager.Settings.Connection",
        (settings,),
        1000,
    );
    if result.is_err() {
        return false;
    }
    true
}

#[allow(dead_code)]
pub fn set_password(path: Path<'static>, password: String) {
    // yes this will be encrypted later
    // TODO encrypt
    let password = Box::new(password) as Box<dyn RefArg>;
    let res = get_connection_settings(path.clone());
    if res.is_err() {
        return;
    }
    let (mut settings,) = res.unwrap();
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

#[allow(dead_code)]
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

pub fn get_access_point_properties(connected: bool, path: Path<'static>) -> AccessPoint {
    let interface = "org.freedesktop.NetworkManager.AccessPoint";
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(
        "org.freedesktop.NetworkManager",
        path.to_string(),
        Duration::from_millis(1000),
    );
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
    let ssid: Vec<u8> = proxy.get(interface, "Ssid").unwrap_or_else(|_| Vec::new());
    let strength: u8 = proxy.get(interface, "Strength").unwrap_or_else(|_| 130);
    let mut associated_connection: Option<Path<'static>> = None;
    let connections = get_stored_connections();
    let mut stored: bool = false;
    for (connection, connection_ssid) in connections {
        if ssid == connection_ssid {
            associated_connection = Some(connection);
            stored = true;
            break;
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
        connected,
        stored,
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
    let devices: Vec<Path<'static>> = proxy
        .get(interface, "Devices")
        .unwrap_or_else(|_| Vec::new());
    let access_point_prop: Path<'static> = proxy
        .get(interface, "SpecificObject")
        .unwrap_or_else(|_| Path::from("/"));
    let connection_type: String = proxy
        .get(interface, "Type")
        .unwrap_or_else(|_| String::from(""));
    let access_point: Option<AccessPoint>;
    if connection_type == "802-11-wireless" {
        access_point = Some(get_access_point_properties(true, access_point_prop));
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
        let res = get_connection_settings(connection.clone());
        if res.is_err() {
            continue;
        }
        let (settings,) = res.unwrap();
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

    pub fn request_scan(&self) {
        let _ = call_system_dbus_method::<(), (Vec<Path<'static>>,)>(
            "org.freedesktop.NetworkManager",
            self.dbus_path.clone(),
            "RequestScan",
            "org.freedesktop.NetworkManager.Device.Wireless",
            (),
            1000,
        );
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
        let mut known_points = HashMap::new();
        if self.access_point.is_some() {
            let connected_access_point = self.access_point.clone().unwrap();
            known_points.insert(connected_access_point.ssid.clone(), 0);
            access_points.push(connected_access_point);
        }
        for label in result {
            let access_point = get_access_point_properties(false, label);
            if known_points.get(&access_point.ssid).is_some() {
                continue;
            }
            known_points.insert(access_point.ssid.clone(), 0);
            access_points.push(access_point);
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
        self.access_point = Some(get_access_point_properties(true, access_point));
    }

    pub fn connect_to_access_point(
        &mut self,
        access_point: AccessPoint,
    ) -> Result<(), ConnectionError> {
        let res = call_system_dbus_method::<
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
        if res.is_err() {
            return Err(ConnectionError {
                method: "connect to",
            });
        }
        let res = res.unwrap();
        let mut result = 1;
        while result == 1 {
            let res = get_system_dbus_property::<(), u32>(
                "org.freedesktop.NetworkManager",
                res.0.clone(),
                "org.freedesktop.NetworkManager.Connection.Active",
                "State",
            );
            if res.is_err() {
                return Err(ConnectionError {
                    method: "Password was wrong",
                });
            }
            result = res.unwrap();
        }
        if result != 2 {
            return Err(ConnectionError {
                method: "Password was wrong",
            });
        }
        let connection = get_associations_of_active_connection(res.0.clone());
        self.connection = Some(res.0);
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
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy(
            "org.freedesktop.NetworkManager",
            Path::from("/org/freedesktop/NetworkManager"),
            Duration::from_millis(1000),
        );
        let result: Result<(Path<'static>, Path<'static>), dbus::Error> = proxy.method_call(
            "org.freedesktop.NetworkManager",
            "AddAndActivateConnection",
            (
                properties,
                self.dbus_path.clone(),
                access_point.dbus_path.clone(),
            ),
        );
        if result.is_ok() {
            let (path, connection) = result.unwrap();
            let mut result = 1;
            while result == 1 {
                let res = get_system_dbus_property::<(), u32>(
                    "org.freedesktop.NetworkManager",
                    connection.clone(),
                    "org.freedesktop.NetworkManager.Connection.Active",
                    "State",
                );
                if res.is_err() {
                    return Err(ConnectionError {
                        method: "Password was wrong",
                    });
                }
                result = res.unwrap();
            }
            if result != 2 {
                return Err(ConnectionError {
                    method: "Password was wrong",
                });
            }
            (self.connection, self.access_point) = (
                Some(connection),
                Some(get_access_point_properties(true, path)),
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

#[derive(Debug)]
pub struct ConnectionStatusChanged {
    pub state: u32,
    pub reason: u32,
}

impl arg::AppendAll for ConnectionStatusChanged {
    fn append(&self, i: &mut arg::IterAppend) {
        arg::RefArg::append(&self.state, i);
        arg::RefArg::append(&self.reason, i);
    }
}

impl arg::ReadAll for ConnectionStatusChanged {
    fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
        Ok(ConnectionStatusChanged {
            state: i.read()?,
            reason: i.read()?,
        })
    }
}

impl dbus::message::SignalArgs for ConnectionStatusChanged {
    const NAME: &'static str = "StateChanged";
    const INTERFACE: &'static str = "org.freedesktop.NetworkManager.Connection.Active";
}
