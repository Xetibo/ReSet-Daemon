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
    arg::{self, prop_cast, PropMap, RefArg, Variant},
    blocking::{stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged, Connection},
    channel::Sender,
    message::SignalArgs,
    nonblock::SyncConnection,
    Message, MethodErr, Path,
};
use re_set_lib::{
    network::{
        network_signals::{AccessPointAdded, AccessPointRemoved},
        network_structures::{AccessPoint, ConnectionError, DeviceType, WifiDevice},
    },
    {ERROR, LOG},
};
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::utils::{DaemonData, MaskedPropMap};

#[derive(Debug)]
pub struct Device {
    pub access_point: Option<AccessPoint>,
    pub connection: Option<Path<'static>>,
    pub dbus_path: Path<'static>,
    pub name: String,
    pub connected: bool,
    pub active_listener: AtomicBool,
}

impl Clone for Device {
    fn clone(&self) -> Self {
        Self {
            access_point: self.access_point.clone(),
            connection: self.connection.clone(),
            dbus_path: self.dbus_path.clone(),
            name: self.name.clone(),
            connected: self.connected,
            active_listener: AtomicBool::new(false),
        }
    }
}

impl Device {
    pub fn new(path: Path<'static>, name: String) -> Self {
        Self {
            access_point: None,
            connection: None,
            dbus_path: path,
            name,
            connected: false,
            active_listener: AtomicBool::new(false),
        }
    }
}

pub fn start_listener(
    connection: Arc<SyncConnection>,
    device: Arc<RwLock<Device>>,
    path: Path<'static>,
    active_listener: Arc<AtomicBool>,
    stop_requested: Arc<AtomicBool>,
) -> Result<(), dbus::Error> {
    let access_point_added_ref = connection.clone();
    let access_point_removed_ref = connection.clone();
    let active_access_point_changed_ref = connection.clone();
    let device_ref = device.clone();
    let manager_ref = device.clone();
    let conn = Connection::new_system().unwrap();
    let access_point_added =
        AccessPointAdded::match_rule(Some(&NETWORK_INTERFACE!().into()), Some(&path))
            .static_clone();
    let access_point_removed =
        AccessPointRemoved::match_rule(Some(&NETWORK_INTERFACE!().into()), Some(&path))
            .static_clone();
    let mut access_point_changed = PropertiesPropertiesChanged::match_rule(
        Some(&NETWORK_INTERFACE!().into()),
        Some(&Path::from(NM_ACCESS_POINT_PATH!())),
    )
    .static_clone();
    access_point_changed.path_is_namespace = true;
    let mut wifi_device_event = PropertiesPropertiesChanged::match_rule(
        Some(&NM_INTERFACE!().into()),
        Some(&Path::from(NM_DEVICES_PATH!())),
    )
    .static_clone();
    wifi_device_event.path_is_namespace = true;
    let active_connection_event = PropertiesPropertiesChanged::match_rule(
        Some(&NM_INTERFACE!().into()),
        Some(&Path::from(NM_PATH!())),
    )
    .static_clone();
    let res = conn.add_match(
        access_point_changed,
        move |ir: PropertiesPropertiesChanged, _, msg| {
            let strength: Option<&u8> = prop_cast(&ir.changed_properties, "Strength");
            let ssid: Option<&Vec<u8>> = prop_cast(&ir.changed_properties, "Ssid");
            if strength.is_none() && ssid.is_none() {
                return true;
            }
            let path = msg.path().unwrap().to_string();
            if path.contains(NM_ACCESS_POINT_PATH!()) {
                let access_point = get_access_point_properties(Path::from(path));
                let msg = Message::signal(
                    &Path::from(DBUS_PATH!()),
                    &NETWORK_INTERFACE!().into(),
                    &"AccessPointChanged".into(),
                )
                .append1(access_point);
                let res = connection.send(msg);
                if let Err(_error) = res {
                    ERROR!(
                        format!("Could not send signal: {:?}", _error),
                        ErrorLevel::PartialBreakage
                    );
                }
            }
            true
        },
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Signal Match on NetworkManager failed: {:?}", _error),
            ErrorLevel::PartialBreakage
        );
        return Err(dbus::Error::new_custom(
            "SignalMatchFailed",
            "Failed to match signal on NetworkManager.",
        ));
    }
    let res = conn.add_match(
        wifi_device_event,
        move |ir: PropertiesPropertiesChanged, _, _| {
            let active_access_point: Option<&Path<'static>> =
                prop_cast(&ir.changed_properties, "ActiveAccessPoint");
            if let Some(active_access_point) = active_access_point {
                let active_access_point = active_access_point.clone();
                if active_access_point != Path::from("/") {
                    let parsed_access_point = get_access_point_properties(active_access_point);
                    let mut device = device_ref.write().unwrap();
                    device.access_point = Some(parsed_access_point.clone());
                    let msg = Message::signal(
                        &Path::from(DBUS_PATH!()),
                        &NETWORK_INTERFACE!().into(),
                        &"WifiDeviceChanged".into(),
                    )
                    .append1(WifiDevice {
                        path: device.dbus_path.clone(),
                        name: device.name.clone(),
                        active_access_point: parsed_access_point.ssid,
                    });
                    let res = active_access_point_changed_ref.send(msg);
                    if let Err(_error) = res {
                        ERROR!(
                            format!("Could not send signal: {:?}", _error),
                            ErrorLevel::PartialBreakage
                        );
                    }
                } else {
                    let device = device_ref.write().unwrap();
                    let msg = Message::signal(
                        &Path::from(DBUS_PATH!()),
                        &NETWORK_INTERFACE!().into(),
                        &"WifiDeviceChanged".into(),
                    )
                    .append1(WifiDevice {
                        path: device.dbus_path.clone(),
                        name: device.name.clone(),
                        active_access_point: Vec::new(),
                    });
                    let res = active_access_point_changed_ref.send(msg);
                    if let Err(_error) = res {
                        ERROR!(
                            format!("Could not send signal: {:?}", _error),
                            ErrorLevel::PartialBreakage
                        );
                    }
                }
            }
            true
        },
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Signal Match on NetworkManager failed: {:?}", _error),
            ErrorLevel::PartialBreakage
        );
        return Err(dbus::Error::new_custom(
            "SignalMatchFailed",
            "Failed to match signal on NetworkManager.",
        ));
    }
    let res = conn.add_match(
        active_connection_event,
        move |ir: PropertiesPropertiesChanged, _, _| {
            let connections: Option<&Vec<Path<'static>>> =
                prop_cast(&ir.changed_properties, "ActiveConnections");
            if let Some(connections) = connections {
                for connection in connections {
                    let (devices, access_point) =
                        get_associations_of_active_connection(connection.clone());
                    let mut current_device = manager_ref.write().unwrap();
                    for device in devices {
                        if device == current_device.dbus_path {
                            current_device.connection = Some(connection.clone());
                            current_device.access_point.clone_from(&access_point);
                        }
                    }
                }
            }
            true
        },
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Signal Match on NetworkManager failed: {:?}", _error),
            ErrorLevel::PartialBreakage
        );
        return Err(dbus::Error::new_custom(
            "SignalMatchFailed",
            "Failed to match signal on NetworkManager.",
        ));
    }
    let res = conn.add_match(access_point_added, move |ir: AccessPointAdded, _, _| {
        let msg = Message::signal(
            &Path::from(DBUS_PATH!()),
            &NETWORK_INTERFACE!().into(),
            &"AccessPointAdded".into(),
        )
        .append1(get_access_point_properties(ir.access_point));
        let res = access_point_added_ref.send(msg);
        if let Err(_error) = res {
            ERROR!(
                format!("Could not send signal: {:?}", _error),
                ErrorLevel::PartialBreakage
            );
        }
        true
    });
    if let Err(_error) = res {
        return Err(dbus::Error::new_custom(
            "SignalMatchFailed",
            &format!("Failed to match signal on NetworkManager: {}", _error),
        ));
    }
    let res = conn.add_match(access_point_removed, move |ir: AccessPointRemoved, _, _| {
        let msg = Message::signal(
            &Path::from(DBUS_PATH!()),
            &NETWORK_INTERFACE!().into(),
            &"AccessPointRemoved".into(),
        )
        .append1(ir.access_point);
        let res = access_point_removed_ref.send(msg);
        if let Err(_error) = res {
            ERROR!(
                format!("Could not send signal: {:?}", _error),
                ErrorLevel::PartialBreakage
            );
        }
        true
    });
    if let Err(_error) = res {
        ERROR!(
            format!("Signal Match on NetworkManager failed: {:?}", _error),
            ErrorLevel::PartialBreakage
        );
        return Err(dbus::Error::new_custom(
            "SignalMatchFailed",
            "Failed to match signal on NetworkManager.",
        ));
    }
    active_listener.store(true, Ordering::SeqCst);
    let mut time = SystemTime::now();
    loop {
        let _ = conn.process(Duration::from_millis(1000))?;
        if stop_requested.load(Ordering::SeqCst) {
            active_listener.store(false, Ordering::SeqCst);
            stop_requested.store(false, Ordering::SeqCst);
            return Ok(());
        }
        // if time.elapsed().unwrap_or(Duration::from_millis(0)) < Duration::from_secs(10) {
        if time.elapsed().unwrap() < Duration::from_secs(10) {
            time = SystemTime::now();
            device.read().unwrap().request_scan();
        }
    }
}

pub fn stop_listener(stop_requested: Arc<AtomicBool>) {
    stop_requested.store(true, Ordering::SeqCst);
}

pub fn get_wifi_devices() -> Vec<Arc<RwLock<Device>>> {
    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        Path::from(NM_PATH!()),
        "GetAllDevices",
        NM_INTERFACE!(),
        (),
        1000,
        (Vec<Path<'static>>,),
    );
    if let Err(_error) = res {
        ERROR!(
            format!(
                "Failed to receive network devices from NetworkManager: {:?}",
                _error
            ),
            ErrorLevel::PartialBreakage
        );
        return Vec::new();
    }
    let (res,) = res.unwrap();
    let devices = Arc::new(RwLock::new(Vec::new()));
    for path in res {
        let loop_ref = devices.clone();
        thread::spawn(move || {
            let name = get_dbus_property!(
                NM_INTERFACE_BASE!(),
                path.clone(),
                NM_DEVICE_INTERFACE!(),
                "Interface",
                String,
            );
            let device_type = get_device_type(path.to_string());
            if device_type == DeviceType::WIFI {
                let mut device = Device::new(path, name.unwrap_or(String::from("empty")));
                device.initialize();
                loop_ref
                    .write()
                    .unwrap()
                    .push(Arc::new(RwLock::new(device)));
            }
        })
        .join()
        .expect("Thread failed at parsing network device");
    }
    let devices = Arc::try_unwrap(devices).unwrap();
    devices.into_inner().unwrap()
}

pub fn get_device_type(path: String) -> DeviceType {
    let res = get_dbus_property!(
        NM_INTERFACE_BASE!(),
        Path::from(path),
        NM_DEVICE_INTERFACE!(),
        "DeviceType",
        u32,
    );

    if res.is_err() {
        return DeviceType::DUMMY;
    }
    let res = res.unwrap();
    DeviceType::from_u32(res)
}

pub fn get_connection_settings(path: Path<'static>) -> Result<MaskedPropMap, dbus::MethodErr> {
    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        path.clone(),
        "GetSettings",
        NM_CONNECTION_INTERFACE!(),
        (),
        1000,
        (HashMap<String, PropMap>,),
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Failed to receive settings from connection: {:?}", _error),
            ErrorLevel::PartialBreakage
        );
        return Err(MethodErr::invalid_arg(
            "Could not get settings from connection",
        ));
    }
    let mut map = res.unwrap().0;
    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        path.clone(),
        "GetSecrets",
        NM_CONNECTION_INTERFACE!(),
        ("802-11-wireless-security",),
        1000,
        (HashMap<String, PropMap>,),
    );
    if res.is_err() {
        // return if not a wifi connection -> hence no wifi secrets
        return Ok(map);
    }

    let security = map.get_mut("802-11-wireless-security");
    if security.is_none() {
        return Ok(map);
    }
    security
        .unwrap()
        .extend(res.unwrap().0.remove("802-11-wireless-security").unwrap());
    Ok(map)
}

pub fn set_connection_settings(path: Path<'static>, settings: HashMap<String, PropMap>) -> bool {
    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        path,
        "Update",
        NM_CONNECTION_INTERFACE!(),
        (settings,),
        1000,
        (HashMap<String, PropMap>,),
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Failed to set settings for connection: {:?}", _error),
            ErrorLevel::Recoverable
        );
        return false;
    }
    true
}

#[allow(dead_code)]
pub fn set_password(path: Path<'static>, password: String) {
    // yes this will be encrypted later
    // TODO: encrypt
    let password = Box::new(password) as Box<dyn RefArg>;
    let res = get_connection_settings(path.clone());
    if let Err(_error) = res {
        ERROR!(
            format!("Failed to get settings for connection: {:?}", _error),
            ErrorLevel::Recoverable
        );
        return;
    }
    let mut settings = res.unwrap();
    settings
        .get_mut("802-11-wireless-security")
        .unwrap()
        .insert("password".to_string(), Variant(password));
    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        path,
        "Update",
        NM_CONNECTION_INTERFACE!(),
        (settings,),
        1000,
        (HashMap<String, PropMap>,),
    );
    res.unwrap();
}

#[allow(dead_code)]
pub fn get_connection_secrets(path: Path<'static>) {
    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        path,
        "GetSecrets",
        NM_CONNECTION_INTERFACE!(),
        ("802-11-wireless-security".to_string(),),
        1000,
        (HashMap<String, PropMap>,),
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Failed to get connection secrets: {:?}", _error),
            ErrorLevel::Recoverable
        );
        return;
    }
    let (_,): (HashMap<String, PropMap>,) = res.unwrap();
}

pub fn get_access_point_properties(path: Path<'static>) -> AccessPoint {
    let conn = dbus_connection!();
    let proxy = conn.with_proxy(
        NM_INTERFACE_BASE!(),
        path.to_string(),
        Duration::from_millis(1000),
    );
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
    let ssid: Vec<u8> = proxy
        .get(NM_ACCESS_POINT_INTERFACE!(), "Ssid")
        .unwrap_or_else(|_| Vec::new());
    let strength: u8 = proxy
        .get(NM_ACCESS_POINT_INTERFACE!(), "Strength")
        .unwrap_or(130);
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
        stored,
    }
}

pub fn get_active_connections() -> Vec<Path<'static>> {
    get_dbus_property!(
        NM_INTERFACE_BASE!(),
        NM_PATH!(),
        NM_INTERFACE!(),
        "ActiveConnections",
        Vec<Path<'static>>,
    )
    .unwrap()
}

pub fn get_associations_of_active_connection(
    path: Path<'static>,
) -> (Vec<Path<'static>>, Option<AccessPoint>) {
    let interface = NM_ACTIVE_CONNECTION_INTERFACE!();
    let conn = dbus_connection!();
    let proxy = conn.with_proxy(NM_INTERFACE_BASE!(), path, Duration::from_millis(1000));
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
    let connection: Path<'static> = proxy
        .get(interface, "Connection")
        .unwrap_or_else(|_| Path::from("/"));
    let devices: Vec<Path<'static>> = proxy
        .get(interface, "Devices")
        .unwrap_or_else(|_| Vec::new());
    let access_point_prop: Path<'static> = proxy
        .get(interface, "SpecificObject")
        .unwrap_or_else(|_| Path::from("/"));
    let connection_type: String = proxy
        .get(interface, "Type")
        .unwrap_or_else(|_| String::from(""));
    let access_point: Option<AccessPoint> = if connection_type == "802-11-wireless" {
        let mut unconnected_access_point = get_access_point_properties(access_point_prop);
        unconnected_access_point.associated_connection = connection;
        unconnected_access_point.stored = true;
        Some(unconnected_access_point)
    } else {
        None
    };
    (devices, access_point)
}

pub fn set_wifi_enabled(enabled: bool, data: &mut DaemonData) -> bool {
    let res = set_dbus_property!(
        NM_INTERFACE_BASE!(),
        Path::from(NM_PATH!()),
        NM_INTERFACE!(),
        "WirelessEnabled",
        enabled,
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Failed to enable WiFi: {:?}", _error),
            ErrorLevel::PartialBreakage
        );
        return false;
    }
    if enabled {
        let devices = get_wifi_devices();
        if devices.is_empty() {
            return false;
        }
        data.current_n_device = devices.last().unwrap().clone();
        data.n_devices = devices;
    }
    true
}

pub fn get_stored_connections() -> Vec<(Path<'static>, Vec<u8>)> {
    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        Path::from(NM_SETTINGS_PATH!()),
        "ListConnections",
        NM_SETTINGS_INTERFACE!(),
        (),
        1000,
        (Vec<Path<'static>>,),
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Failed to get connection settings: {:?}", _error),
            ErrorLevel::Recoverable
        );
        return Vec::new();
    }
    let (result,) = res.ok().unwrap();
    let mut wifi_connections = Vec::new();
    for connection in result {
        let result = get_connection_settings(connection.clone());
        if let Err(_error) = result {
            ERROR!(
                format!("Failed to get connection settings: {:?}", _error),
                ErrorLevel::Recoverable
            );
            continue;
        }
        let settings = result.unwrap();
        let settings = settings.get("802-11-wireless");
        if let Some(settings) = settings {
            let x = &Vec::new();
            let ssid: &Vec<u8> = arg::prop_cast(settings, "ssid").unwrap_or(x);
            let ssid = ssid.clone();
            wifi_connections.push((connection, ssid));
        }
    }
    wifi_connections
}

pub fn disconnect_from_access_point(connection: Path<'static>) -> Result<(), ConnectionError> {
    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        Path::from(NM_PATH!()),
        "DeactivateConnection",
        NM_INTERFACE!(),
        (connection,),
        1000,
        (),
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Failed to disconnect from connection: {}", _error),
            ErrorLevel::Recoverable
        );
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
        let res = dbus_method!(
            NM_INTERFACE_BASE!(),
            self.dbus_path.clone(),
            "RequestScan",
            NM_DEVICE_WIRELESS_INTERFACE!(),
            (PropMap::new(),),
            1000,
            (),
        );
        if let Err(_error) = res {
            ERROR!(
                format!("Failed to request scan from WiFi device: {}", _error),
                ErrorLevel::Recoverable
            );
        }
    }

    pub fn get_access_points(&self) -> Vec<AccessPoint> {
        let res = dbus_method!(
            NM_INTERFACE_BASE!(),
            self.dbus_path.clone(),
            "GetAllAccessPoints",
            NM_DEVICE_WIRELESS_INTERFACE!(),
            (),
            1000,
            (Vec<Path<'static>>,),
        );
        if let Err(_error) = res {
            ERROR!(
                format!(
                    "Failed to receive access points from WiFi device: {:?}",
                    _error
                ),
                ErrorLevel::PartialBreakage
            );
            return Vec::new();
        }
        let (res,) = res.unwrap();
        let access_points = Arc::new(RwLock::new(Vec::new()));
        let known_points = Arc::new(RwLock::new(HashMap::new()));
        if self.access_point.is_some() {
            let connected_access_point = self.access_point.clone().unwrap();
            known_points
                .write()
                .unwrap()
                .insert(connected_access_point.ssid.clone(), 0);
            access_points.write().unwrap().push(connected_access_point);
        }

        let mut threads = Vec::new();
        for label in res {
            let known_points_ref = known_points.clone();
            let access_points_ref = access_points.clone();
            threads.push(thread::spawn(move || {
                let access_point = get_access_point_properties(label);
                if known_points_ref
                    .read()
                    .unwrap()
                    .contains_key(&access_point.ssid)
                {
                    return;
                }
                known_points_ref
                    .write()
                    .unwrap()
                    .insert(access_point.ssid.clone(), 0);
                access_points_ref.write().unwrap().push(access_point);
            }));
        }
        for thread in threads {
            thread.join().expect("Could not spawn thread");
        }
        Arc::try_unwrap(access_points)
            .unwrap()
            .into_inner()
            .unwrap()
    }

    #[allow(dead_code)]
    pub fn set_active_access_point(&mut self) {
        if self.dbus_path.is_empty() {
            return;
        }
        let interface = NM_DEVICE_INTERFACE!();
        let conn = dbus_connection!();
        let proxy = conn.with_proxy(
            NM_INTERFACE_BASE!(),
            self.dbus_path.clone(),
            Duration::from_millis(1000),
        );
        use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
        let access_point: Path<'static> = proxy.get(interface, "ActiveAccessPoint").unwrap();
        self.access_point = Some(get_access_point_properties(access_point));
    }

    pub fn connect_to_access_point(
        &mut self,
        access_point: AccessPoint,
    ) -> Result<(), ConnectionError> {
        if self.dbus_path.is_empty() {
            ERROR!(
                "Tried to connect to access point with invalid device: {:?}",
                ErrorLevel::PartialBreakage
            );
            return Err(ConnectionError {
                method: "WifiDevice is not valid",
            });
        }
        let res = dbus_method!(
            NM_INTERFACE_BASE!(),
            Path::from(NM_PATH!()),
            "ActivateConnection",
            NM_INTERFACE!(),
            (
                access_point.associated_connection,
                self.dbus_path.clone(),
                access_point.dbus_path.clone(),
            ),
            1000,
            (Path<'static>,),
        );
        if let Err(_error) = res {
            ERROR!(
                format!("Failed to activate connection: {:?}", _error),
                ErrorLevel::Recoverable
            );
            return Err(ConnectionError {
                method: "connect to",
            });
        }
        let result = res.unwrap();
        let mut res_number = 1;
        while res_number == 1 {
            let path = result.0.clone();
            let checked_result = get_dbus_property!(
                NM_INTERFACE_BASE!(),
                path.clone(),
                NM_ACTIVE_CONNECTION_INTERFACE!(),
                "State",
                u32,
            );
            if let Err(_error) = checked_result {
                ERROR!(
                    format!("Failed to get status of WiFi: {:?}", _error),
                    ErrorLevel::PartialBreakage
                );
                return Err(ConnectionError {
                    method: "Failed to receive WiFi status",
                });
            }
            res_number = checked_result.unwrap();
        }
        if res_number != 2 {
            LOG!(format!(
                "Wrong password entered for connection: {}.",
                result.0
            ));
            return Err(ConnectionError {
                method: "Password was wrong",
            });
        }
        let connection = get_associations_of_active_connection(result.0.clone());
        self.connection = Some(result.0);
        self.access_point = connection.1;
        self.connected = true;
        Ok(())
    }

    pub fn add_and_connect_to_access_point(
        &mut self,
        access_point: AccessPoint,
        password: String,
    ) -> Result<(), ConnectionError> {
        if self.dbus_path.is_empty() {
            ERROR!(
                "Tried to connect to access point with invalid device.",
                ErrorLevel::PartialBreakage
            );
            return Err(ConnectionError {
                method: "WifiDevice is not valid",
            });
        }
        let mut properties = HashMap::new();
        properties.insert("802-11-wireless-security".to_string(), PropMap::new());
        let password = Box::new(password) as Box<dyn RefArg>;
        properties
            .get_mut("802-11-wireless-security")
            .unwrap()
            .insert("psk".to_string(), Variant(password));
        let res = dbus_method!(
            NM_INTERFACE_BASE!(),
            Path::from(NM_PATH!()),
            "AddAndActivateConnection",
            NM_INTERFACE!(),
            (
                properties,
                self.dbus_path.clone(),
                access_point.dbus_path.clone(),
            ),
            1000,
            (Path<'static>, Path<'static>),
        );
        if let Ok(res) = res {
            let (path, connection) = res;
            let mut res = 1;
            while res == 1 {
                let result = get_dbus_property!(
                    NM_INTERFACE_BASE!(),
                    connection.clone(),
                    NM_ACTIVE_CONNECTION_INTERFACE!(),
                    "State",
                    u32,
                );
                if result.is_err() {
                    LOG!(format!("Wrong password entered for connection: {}.", path));
                    return Err(ConnectionError {
                        method: "Password was wrong",
                    });
                }
                res = result.unwrap();
            }
            if res != 2 {
                LOG!(format!("Wrong password entered for connection: {}.", path));
                return Err(ConnectionError {
                    method: "Password was wrong",
                });
            }
            (self.connection, self.access_point) =
                (Some(connection), Some(get_access_point_properties(path)));
            return Ok(());
        }
        LOG!(format!("Failed to connect to {}.", access_point.dbus_path));
        Err(ConnectionError {
            method: "connect to",
        })
    }

    pub fn disconnect_from_current(&mut self) -> Result<(), ConnectionError> {
        if self.dbus_path.is_empty() {
            return Err(ConnectionError {
                method: "WifiDevice is not valid",
            });
        }
        let res = get_dbus_property!(
            NM_INTERFACE_BASE!(),
            Path::from(NM_PATH!()),
            NM_INTERFACE!(),
            "ActiveConnections",
            Vec<Path<'static>>,
        );
        if let Err(_error) = res {
            ERROR!(
                format!("Tried to disconnect from access point: {:?}", _error),
                ErrorLevel::Recoverable
            );
            return Err(ConnectionError {
                method: "disconnect from",
            });
        }
        for connection in res.unwrap() {
            let (devices, _) = get_associations_of_active_connection(connection.clone());
            for device in devices {
                if device == self.dbus_path {
                    let res = disconnect_from_access_point(connection);
                    if let Err(_error) = res {
                        ERROR!(
                            format!("Tried to disconnect from access point: {:?}", _error),
                            ErrorLevel::Recoverable
                        );
                        return Err(ConnectionError {
                            method: "disconnect from",
                        });
                    }
                    self.connected = false;
                    self.access_point = None;
                    self.connection = None;
                    break;
                }
            }
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
