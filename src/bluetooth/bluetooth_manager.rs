use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicI8, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use dbus::{
    arg::{self, prop_cast, PropMap},
    blocking::{stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged, Connection},
    channel::Sender,
    message::SignalArgs,
    nonblock::SyncConnection,
    Message, Path,
};
use dbus_tokio::connection;
use re_set_lib::{
    bluetooth::{
        bluetooth_signals::{BluetoothDeviceAdded, BluetoothDeviceRemoved},
        bluetooth_structures::{BluetoothAdapter, BluetoothDevice},
    },
    {ERROR, LOG},
};
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::utils::{convert_bluetooth_map_bool, MaskedPropMap};

#[allow(dead_code)]
#[derive(Clone)]
pub struct BluetoothInterface {
    pub adapters: Vec<Path<'static>>,
    pub current_adapter: Path<'static>,
    devices: HashMap<Path<'static>, BluetoothDevice>,
    enabled: bool,
    registered: bool,
    in_discovery: Arc<AtomicBool>,
    connection: Arc<SyncConnection>,
}

pub struct BluetoothAgent {
    pub in_progress: bool,
}

impl BluetoothAgent {
    pub fn new() -> Self {
        Self { in_progress: false }
    }
}

impl Default for BluetoothAgent {
    fn default() -> Self {
        Self::new()
    }
}

fn get_objects() -> HashMap<Path<'static>, HashMap<String, PropMap>> {
    let res = dbus_method!(
        BLUEZ_INTERFACE!(),
        "/",
        "GetManagedObjects",
        "org.freedesktop.DBus.ObjectManager",
        (),
        1000,
        (HashMap<Path<'static>, HashMap<String, PropMap>>,),
    );
    if let Err(error) = res {
        ERROR!(
            format!("Could not to get bluetooth objects {}", error),
            ErrorLevel::PartialBreakage
        );
        return HashMap::new();
    }
    res.unwrap().0
}

pub fn convert_device(path: &Path<'static>, map: &MaskedPropMap) -> Option<BluetoothDevice> {
    let map = map.get(BLUEZ_DEVICE_INTERFACE!());
    map?;
    let map = map.unwrap();
    bluetooth_device_from_map(path, map)
}

pub fn bluetooth_device_from_map(path: &Path<'static>, map: &PropMap) -> Option<BluetoothDevice> {
    if map.is_empty() {
        return None;
    }
    let rssi: i16;
    let rssi_opt = map.get("RSSI");
    if let Some(rssi_opt) = rssi_opt {
        rssi = *arg::cast::<i16>(&rssi_opt.0).unwrap();
    } else {
        rssi = -1;
    }
    let name_opt: Option<&String> = prop_cast(map, "Name");
    let name = if let Some(name_opt) = name_opt {
        name_opt.clone()
    } else {
        String::from("")
    };
    let alias_opt: Option<&String> = prop_cast(map, "Alias");
    let alias = if let Some(alias_opt) = alias_opt {
        alias_opt.clone()
    } else {
        String::from("")
    };
    let adapter_opt: Option<&Path<'static>> = prop_cast(map, "Adapter");
    let adapter = if let Some(adapter_opt) = adapter_opt {
        adapter_opt.clone()
    } else {
        return None;
    };
    let trusted = convert_bluetooth_map_bool(map.get("Trusted"));
    let bonded = convert_bluetooth_map_bool(map.get("Bonded"));
    let blocked = convert_bluetooth_map_bool(map.get("Blocked"));
    let paired = convert_bluetooth_map_bool(map.get("Paired"));
    let connected_opt: Option<&bool> = prop_cast(map, "Connected");
    let icon_opt: Option<&String> = prop_cast(map, "Icon");
    let icon = if let Some(icon_opt) = icon_opt {
        icon_opt.clone()
    } else {
        String::from("")
    };
    let address_opt: Option<&String> = prop_cast(map, "Address");
    let address = if let Some(address_opt) = address_opt {
        address_opt.clone()
    } else {
        String::from("")
    };
    Some(BluetoothDevice {
        path: path.clone(),
        rssi,
        name,
        alias,
        adapter,
        trusted,
        bonded,
        paired,
        blocked,
        connected: *connected_opt.unwrap_or(&false),
        icon,
        address,
    })
}

pub fn adapter_from_map(path: &Path<'static>, map: &PropMap) -> BluetoothAdapter {
    let alias = arg::cast::<String>(&map.get("Alias").unwrap().0)
        .unwrap()
        .clone();
    let powered = *arg::cast::<bool>(&map.get("Powered").unwrap().0).unwrap();
    let discoverable = *arg::cast::<bool>(&map.get("Discoverable").unwrap().0).unwrap();
    let pairable = *arg::cast::<bool>(&map.get("Pairable").unwrap().0).unwrap();
    BluetoothAdapter {
        path: path.clone(),
        alias,
        powered,
        discoverable,
        pairable,
    }
}

pub fn get_bluetooth_adapter(path: &Path<'static>) -> BluetoothAdapter {
    let res = dbus_method!(
        BLUEZ_INTERFACE!(),
        path.clone(),
        "GetAll",
        "org.freedesktop.DBus.Properties",
        (BLUEZ_ADAPTER_INTERFACE!(),),
        1000,
        (PropMap,),
    );
    let map = if let Ok(res) = res {
        res.0
    } else {
        println!("f");
        PropMap::new()
    };
    adapter_from_map(path, &map)
}

pub fn get_connections() -> Vec<re_set_lib::bluetooth::bluetooth_structures::BluetoothDevice> {
    let mut devices = Vec::new();
    let res = get_objects();
    for (path, map) in res.iter() {
        let device = convert_device(path, map);
        if let Some(device) = device {
            devices.push(device);
        }
    }
    devices
}

#[allow(dead_code)]
// pairing is currently not used
// TODO handle pairing according to bluetooth rules
impl BluetoothInterface {
    pub fn empty() -> Self {
        Self {
            adapters: Vec::new(),
            current_adapter: Path::from("/"),
            devices: HashMap::new(),
            enabled: false,
            registered: false,
            in_discovery: Arc::new(AtomicBool::new(false)),
            connection: connection::new_session_sync().unwrap().1,
        }
    }

    pub fn create(conn: Arc<SyncConnection>) -> Option<Self> {
        let mut adapters = Vec::new();
        let res = get_objects();
        for (path, map) in res.iter() {
            let map = map.get(BLUEZ_ADAPTER_INTERFACE!());
            if map.is_none() {
                continue;
            }
            adapters.push(path.clone());
        }
        if adapters.is_empty() {
            return None;
        }
        let current_adapter = adapters.last().unwrap().clone();
        Some(Self {
            adapters,
            current_adapter,
            devices: HashMap::new(),
            enabled: false,
            registered: false,
            in_discovery: Arc::new(AtomicBool::new(false)),
            connection: conn,
        })
    }

    pub fn start_bluetooth_listener(
        &self,
        active_listener: Arc<AtomicBool>,
        scan_request: Arc<AtomicI8>,
        scan_active: Arc<AtomicBool>,
        stop_requested: Arc<AtomicBool>,
    ) -> bool {
        let path = self.current_adapter.clone();
        let added_ref = self.connection.clone();
        let removed_ref = self.connection.clone();
        let changed_ref = self.connection.clone();

        if active_listener.load(Ordering::SeqCst) {
            return false;
        }
        thread::spawn(move || {
            let conn = dbus_connection!();
            let bluetooth_device_added =
                BluetoothDeviceAdded::match_rule(Some(&BLUEZ_INTERFACE!().into()), None)
                    .static_clone();
            let bluetooth_device_removed =
                BluetoothDeviceRemoved::match_rule(Some(&BLUEZ_INTERFACE!().into()), None)
                    .static_clone();
            let mut bluetooth_device_changed = PropertiesPropertiesChanged::match_rule(
                Some(&BLUEZ_INTERFACE!().into()),
                Some(&path.clone()),
            )
            .static_clone();
            bluetooth_device_changed.path_is_namespace = true;
            let res = conn.add_match(
                bluetooth_device_added,
                move |ir: BluetoothDeviceAdded, _, _| {
                    let device = convert_device(&ir.object, &ir.interfaces);
                    if let Some(device) = device {
                        let msg = Message::signal(
                            &Path::from(DBUS_PATH!()),
                            &BLUETOOTH_INTERFACE!().into(),
                            &"BluetoothDeviceAdded".into(),
                        )
                        .append1(device);
                        let res = added_ref.send(msg);
                        if let Err(error) = res {
                            ERROR!(
                                format!("Could not send signal: {:?}", error),
                                ErrorLevel::PartialBreakage
                            );
                        }
                    }
                    true
                },
            );
            if let Err(error) = res {
                ERROR!(
                    format!("Failed to match signal on bluez {:?}", error),
                    ErrorLevel::Critical
                );
                return Err(dbus::Error::new_custom(
                    "SignalMatchFailed",
                    "Failed to match signal on bluez.",
                ));
            }
            let res = conn.add_match(
                bluetooth_device_removed,
                move |ir: BluetoothDeviceRemoved, _, _| {
                    let msg = Message::signal(
                        &Path::from(DBUS_PATH!()),
                        &BLUETOOTH_INTERFACE!().into(),
                        &"BluetoothDeviceRemoved".into(),
                    )
                    .append1(ir.object);
                    let res = removed_ref.send(msg);
                    if let Err(error) = res {
                        ERROR!(
                            format!("Could not send signal {:?}", error),
                            ErrorLevel::PartialBreakage
                        );
                    }
                    true
                },
            );
            if let Err(error) = res {
                ERROR!(
                    format!("Failed to match signal on bluez {:?}", error),
                    ErrorLevel::Critical
                );
                return Err(dbus::Error::new_custom(
                    "SignalMatchFailed",
                    "Failed to match signal on bluez.",
                ));
            }
            let res = conn.add_match(
                bluetooth_device_changed,
                move |ir: PropertiesPropertiesChanged, _, msg| {
                    if ir.interface_name != BLUEZ_DEVICE_INTERFACE!() {
                        // Here we only want to match on bluetooth device signals, the rest can be
                        // ignored.
                        return true;
                    }
                    if let Some(path) = msg.path() {
                        let string = path.to_string();
                        let path = Path::from(string);
                        let map = get_bluetooth_device_properties(&path);
                        let device_opt = bluetooth_device_from_map(&path, &map);

                        if let Some(device) = device_opt {
                            let msg = Message::signal(
                                &Path::from(DBUS_PATH!()),
                                &BLUETOOTH_INTERFACE!().into(),
                                &"BluetoothDeviceChanged".into(),
                            )
                            .append1(device);
                            let res = changed_ref.clone().send(msg);
                            if let Err(error) = res {
                                ERROR!(
                                    format!("Could not send signal: {:?}", error),
                                    ErrorLevel::PartialBreakage
                                );
                            }
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                },
            );
            if let Err(error) = res {
                ERROR!(
                    format!("Failed to match signal on bluez: {:?}", error),
                    ErrorLevel::Critical
                );
                return Err(dbus::Error::new_custom(
                    "SignalMatchFailed",
                    "Failed to match signal on bluez.",
                ));
            }
            let other = Connection::new_system().unwrap();
            let proxy = other.with_proxy(
                BLUEZ_INTERFACE!(),
                path.clone(),
                Duration::from_millis(1000),
            );
            let res: Result<(), dbus::Error> =
                proxy.method_call(BLUEZ_ADAPTER_INTERFACE!(), "StartDiscovery", ());
            active_listener.store(true, Ordering::SeqCst);
            scan_active.store(true, Ordering::SeqCst);
            loop {
                let _ = conn.process(Duration::from_millis(1000))?;
                if stop_requested.load(Ordering::SeqCst) {
                    scan_request.store(0, Ordering::SeqCst);
                    active_listener.store(false, Ordering::SeqCst);
                    stop_requested.store(false, Ordering::SeqCst);
                    let res: Result<(), dbus::Error> =
                        proxy.method_call(BLUEZ_ADAPTER_INTERFACE!(), "StopDiscovery", ());
                    if let Err(error) = res {
                        ERROR!(
                            format!("Failed to stop bluetooth discovery: {:?}", error),
                            ErrorLevel::Critical
                        );
                    } else {
                        scan_active.store(false, Ordering::SeqCst);
                    }
                    break;
                }
                if scan_request.load(Ordering::SeqCst) == 1 {
                    scan_request.store(0, Ordering::SeqCst);
                    let res: Result<(), dbus::Error> =
                        proxy.method_call(BLUEZ_ADAPTER_INTERFACE!(), "StartDiscovery", ());
                    if let Err(error) = res {
                        ERROR!(
                            format!("Failed to start bluetooth discovery: {:?}", error),
                            ErrorLevel::Critical
                        );
                    } else {
                        scan_active.store(true, Ordering::SeqCst);
                    }
                } else if scan_request.load(Ordering::SeqCst) == 2 {
                    scan_request.store(0, Ordering::SeqCst);
                    let res: Result<(), dbus::Error> =
                        proxy.method_call(BLUEZ_ADAPTER_INTERFACE!(), "StopDiscovery", ());
                    if let Err(error) = res {
                        ERROR!(
                            format!("Failed to stop bluetooth discovery: {:?}", error),
                            ErrorLevel::Critical
                        );
                    } else {
                        scan_active.store(false, Ordering::SeqCst);
                    }
                }
            }
            res
        });
        true
    }

    pub fn connect_to(&self, device: Path<'static>) {
        thread::spawn(move || {
            let res = dbus_method!(
                BLUEZ_INTERFACE!(),
                device.clone(),
                "Connect",
                BLUEZ_DEVICE_INTERFACE!(),
                (),
                10000,
                (),
            );
            if let Err(error) = res {
                ERROR!(
                    format!(
                        "Failed to connect to bluetooth device: {} with error: {}",
                        device, error
                    ),
                    ErrorLevel::Critical
                );
            }
        });
    }

    pub fn pair_with(&mut self, device: Path<'static>) {
        if !self.registered {
            self.register_agent();
        }
        thread::spawn(move || {
            let res = dbus_method!(
                BLUEZ_INTERFACE!(),
                device.clone(),
                "Pair",
                BLUEZ_DEVICE_INTERFACE!(),
                (),
                10000,
                (),
            );
            if let Err(error) = res {
                ERROR!(
                    format!(
                        "Failed to pair with bluetooth device: {} with error {}",
                        device, error
                    ),
                    ErrorLevel::Critical
                );
            }
        });
    }

    pub fn disconnect(&self, device: Path<'static>) -> Result<(), dbus::Error> {
        dbus_method!(
            BLUEZ_INTERFACE!(),
            device,
            "Disconnect",
            BLUEZ_DEVICE_INTERFACE!(),
            (),
            1000,
            (),
        )
    }

    pub fn register_agent(&mut self) -> bool {
        if self.registered {
            return false;
        }
        let res = dbus_method!(
            BLUEZ_INTERFACE!(),
            Path::from(BLUEZ_PATH!()),
            "RegisterAgent",
            BLUEZ_AGENT_INTERFACE!(),
            (Path::from(DBUS_PATH!()), "DisplayYesNo"),
            1000,
            (),
        );
        if let Err(error) = res {
            ERROR!(
                format!("Failed to register bluetooth agent: {}", error),
                ErrorLevel::PartialBreakage
            );
            return false;
        }
        self.registered = true;
        true
    }

    pub fn unregister_agent(&mut self) -> bool {
        if !self.registered {
            return false;
        }
        let res = dbus_method!(
            BLUEZ_INTERFACE!(),
            Path::from(BLUEZ_PATH!()),
            "UnregisterAgent",
            BLUEZ_AGENT_INTERFACE!(),
            (Path::from(DBUS_PATH!()),),
            1000,
            (Path<'static>,),
        );
        if let Err(error) = res {
            ERROR!(
                format!("Failed to unregister bluetooth agent {}", error),
                ErrorLevel::PartialBreakage
            );
            return false;
        }
        self.registered = false;
        true
    }

    pub fn start_bluetooth_discovery(&self, scan_active: Arc<AtomicBool>) {
        if scan_active.load(Ordering::SeqCst) {
            LOG!("Failed to start bluetooth, already active");
            return;
        }
        let res = dbus_method!(
            BLUEZ_INTERFACE!(),
            self.current_adapter.clone(),
            "StartDiscovery",
            BLUEZ_ADAPTER_INTERFACE!(),
            (),
            1000,
            (),
        );
        if let Err(error) = res {
            ERROR!(
                format!("Failed to start bluetooth discovery: {}", error),
                ErrorLevel::PartialBreakage
            );
        } else {
            scan_active.store(true, Ordering::SeqCst);
        }
    }

    pub fn stop_bluetooth_discovery(&self, scan_active: Arc<AtomicBool>) {
        let res = dbus_method!(
            BLUEZ_INTERFACE!(),
            self.current_adapter.clone(),
            "StopDiscovery",
            BLUEZ_ADAPTER_INTERFACE!(),
            (),
            1000,
            (),
        );
        if let Err(error) = res {
            ERROR!(
                format!("Could not stop bluetooth discovery {}", error),
                ErrorLevel::PartialBreakage
            );
        } else {
            scan_active.store(false, Ordering::SeqCst);
        }
    }

    pub fn remove_device_pairing(&self, path: Path<'static>) -> Result<(), dbus::Error> {
        dbus_method!(
            BLUEZ_INTERFACE!(),
            self.current_adapter.clone(),
            "RemoveDevice",
            BLUEZ_ADAPTER_INTERFACE!(),
            (path,),
            1000,
            (),
        )
    }
}

fn get_bluetooth_device_properties(path: &Path<'static>) -> PropMap {
    let res = dbus_method!(
        BLUEZ_INTERFACE!(),
        path,
        "GetAll",
        "org.freedesktop.DBus.Properties",
        (BLUEZ_DEVICE_INTERFACE!(),),
        1000,
        (PropMap,),
    );
    if let Err(error) = res {
        ERROR!(
            format!(
                "Failed to get properties of bluetooth device: {} with error: {}",
                path, error
            ),
            ErrorLevel::Recoverable
        );
        return PropMap::new();
    }
    res.unwrap().0
}

pub fn set_adapter_enabled(path: Path<'static>, enabled: bool) -> bool {
    let res = set_dbus_property!(
        BLUEZ_INTERFACE!(),
        path.clone(),
        BLUEZ_ADAPTER_INTERFACE!(),
        "Powered",
        enabled,
    );
    if let Err(error) = res {
        ERROR!(
            format!(
                "Failed to set enabled mode on bluetooth adapter {} to: {} with error: {}",
                path, enabled, error
            ),
            ErrorLevel::Recoverable
        );
        return false;
    }
    true
}

pub fn set_adapter_discoverable(path: Path<'static>, enabled: bool) -> bool {
    let res = set_dbus_property!(
        BLUEZ_INTERFACE!(),
        path.clone(),
        BLUEZ_ADAPTER_INTERFACE!(),
        "Discoverable",
        enabled,
    );
    if let Err(error) = res {
        ERROR!(
            format!(
                "Failed to set discoverability mode on bluetooth adapter {} to: {} with error: {}",
                path, enabled, error
            ),
            ErrorLevel::Recoverable
        );
        return false;
    }
    true
}

pub fn set_adapter_pairable(path: Path<'static>, enabled: bool) -> bool {
    let res = set_dbus_property!(
        BLUEZ_INTERFACE!(),
        path.clone(),
        BLUEZ_ADAPTER_INTERFACE!(),
        "Pairable",
        enabled,
    );
    if let Err(error) = res {
        ERROR!(
            format!(
                "Failed to set pairability mode on bluetooth adapter {} to: {} with error: {}",
                path, enabled, error
            ),
            ErrorLevel::Recoverable
        );
        return false;
    }
    true
}

pub fn get_all_bluetooth_adapters() -> Vec<BluetoothAdapter> {
    let mut adapters = Vec::new();
    let objects = get_objects();
    for (path, map) in objects {
        if path.contains(BLUEZ_CONTAINS_PATH!()) && map.contains_key(BLUEZ_ADAPTER_INTERFACE!()) {
            adapters.push(adapter_from_map(
                &path,
                map.get(BLUEZ_ADAPTER_INTERFACE!()).unwrap(),
            ));
        }
    }
    adapters
}

pub fn get_all_bluetooth_devices() -> Vec<BluetoothDevice> {
    let mut devices = Vec::new();
    let objects = get_objects();
    for (path, map) in objects {
        if path.contains(BLUEZ_CONTAINS_PATH!()) && map.contains_key(BLUEZ_DEVICE_INTERFACE!()) {
            devices.push(
                bluetooth_device_from_map(&path, map.get(BLUEZ_DEVICE_INTERFACE!()).unwrap())
                    .unwrap(),
            );
        }
    }
    devices
}

// command needed to understand anything about bluetooth
// dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
// /org/freedesktop/DBus org.freedesktop.DBus.ListNames | grep -v '":'
