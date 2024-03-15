use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use dbus::{
    arg::{self, prop_cast, PropMap, RefArg, Variant},
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
    utils::macros::ErrorLevel,
    {write_log_to_file, ERROR, LOG},
};

use crate::utils::{convert_bluetooth_map_bool, FullMaskedPropMap, MaskedPropMap};

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

fn get_objects(
    interface: &'static str,
    path: &'static str,
) -> Result<(FullMaskedPropMap,), dbus::Error> {
    dbus_method!(
        interface,
        Path::from(path),
        "GetManagedObjects",
        "org.freedesktop.DBus.ObjectManager",
        (),
        1000,
        (HashMap<Path<'static>, HashMap<String, HashMap<String, Variant<Box<dyn RefArg>>>>>,),
    )
}

pub fn convert_device(path: &Path<'static>, map: &MaskedPropMap) -> Option<BluetoothDevice> {
    let map = map.get("org.bluez.Device1");
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
        ("org.bluez.Adapter1",),
        1000,
        (PropMap,),
    );
    let map = if let Ok(res) = res {
        res.0
    } else {
        PropMap::new()
    };
    adapter_from_map(path, &map)
}

pub fn get_connections() -> Vec<re_set_lib::bluetooth::bluetooth_structures::BluetoothDevice> {
    let mut devices = Vec::new();
    let res = get_objects("org.bluez", "/");
    if res.is_err() {
        return devices;
    }
    let (res,) = res.unwrap();
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
        let res = get_objects(BLUEZ_INTERFACE!(), "/");
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Could not get bluetooth objects",
                ErrorLevel::PartialBreakage
            );
            return None;
        }
        let (res,) = res.unwrap();
        for (path, map) in res.iter() {
            let map = map.get("org.bluez.Adapter1");
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
        active_scan: Arc<AtomicBool>,
        stop_requested: Arc<AtomicBool>,
    ) -> bool {
        let path = self.current_adapter.clone();
        let added_ref = self.connection.clone();
        let removed_ref = self.connection.clone();
        let changed_ref = self.connection.clone();

        if active_listener.load(Ordering::SeqCst) {
            active_scan.store(true, Ordering::SeqCst);
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
                        if res.is_err() {
                            ERROR!(
                                "/tmp/reset_daemon_log",
                                "Could not send signal",
                                ErrorLevel::PartialBreakage
                            );
                        }
                    }
                    true
                },
            );
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Failed to match signal on bluez\n",
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
                    if res.is_err() {
                        ERROR!(
                            "/tmp/reset_daemon_log",
                            "Could not send signal",
                            ErrorLevel::PartialBreakage
                        );
                    }
                    true
                },
            );
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Failed to match signal on bluez\n",
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
                    if ir.interface_name != "org.bluez.Device1" {
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
                            if res.is_err() {
                                ERROR!(
                                    "/tmp/reset_daemon_log",
                                    "Could not send signal",
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
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Failed to match signal on bluez\n",
                    ErrorLevel::Critical
                );
                return Err(dbus::Error::new_custom(
                    "SignalMatchFailed",
                    "Failed to match signal on bluez.",
                ));
            }
            let other = Connection::new_system().unwrap();
            let proxy = other.with_proxy("org.bluez", path.clone(), Duration::from_millis(1000));
            let res: Result<(), dbus::Error> =
                proxy.method_call("org.bluez.Adapter1", "StartDiscovery", ());
            active_listener.store(true, Ordering::SeqCst);
            active_scan.store(true, Ordering::SeqCst);
            loop {
                let _ = conn.process(Duration::from_millis(1000))?;
                if stop_requested.load(Ordering::SeqCst) {
                    active_scan.store(false, Ordering::SeqCst);
                    active_listener.store(false, Ordering::SeqCst);
                    stop_requested.store(false, Ordering::SeqCst);
                    let res: Result<(), dbus::Error> =
                        proxy.method_call("org.bluez.Adapter1", "StopDiscovery", ());
                    if res.is_err() {
                        ERROR!(
                            "/tmp/reset_daemon_log",
                            "Failed to start bluetooth discovery\n",
                            ErrorLevel::Critical
                        );
                    }
                    break;
                }
                if active_scan.load(Ordering::SeqCst) {
                    let res: Result<(), dbus::Error> =
                        proxy.method_call("org.bluez.Adapter1", "StartDiscovery", ());
                    if res.is_err() {
                        ERROR!(
                            "/tmp/reset_daemon_log",
                            "Failed to start bluetooth discovery\n",
                            ErrorLevel::Critical
                        );
                    }
                } else if !active_scan.load(Ordering::SeqCst) {
                    let res: Result<(), dbus::Error> =
                        proxy.method_call("org.bluez.Adapter1", "StopDiscovery", ());
                    if res.is_err() {
                        ERROR!(
                            "/tmp/reset_daemon_log",
                            "Failed to stop bluetooth discovery\n",
                            ErrorLevel::Critical
                        );
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
                "org.bluez.Device1",
                (),
                10000,
                (),
            );
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    format!("Failed to connect to bluetooth device: {}\n", device),
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
                "org.bluez.Device1",
                (),
                10000,
                (),
            );
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    format!("Failed to pair with bluetooth device: {}\n", device),
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
            "org.bluez.Device1",
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
            "org.bluez.AgentManager1",
            (Path::from(DBUS_PATH!()), "DisplayYesNo"),
            1000,
            (),
        );
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Failed to register bluetooth agent\n",
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
            "org.bluez.AgentManager1",
            (Path::from(DBUS_PATH!()),),
            1000,
            (Path<'static>,),
        );
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Failed to unregister bluetooth agent\n",
                ErrorLevel::PartialBreakage
            );
            return false;
        }
        self.registered = false;
        true
    }

    pub fn start_bluetooth_discovery(&self, scan_active: Arc<AtomicBool>) {
        if scan_active.load(Ordering::SeqCst) {
            LOG!(
                "/tmp/reset_daemon_log",
                "Failed to start bluetooth, already active\n"
            );
        }
        scan_active.store(false, Ordering::SeqCst);
        let res = dbus_method!(
            BLUETOOTH_INTERFACE!(),
            self.current_adapter.clone(),
            "StartDiscovery",
            "org.bluez.Adapter1",
            (),
            1000,
            (),
        );
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Failed to start bluetooth discovery\n",
                ErrorLevel::PartialBreakage
            );
        }
    }

    pub fn stop_bluetooth_discovery(&self) {
        let res = dbus_method!(
            BLUEZ_INTERFACE!(),
            self.current_adapter.clone(),
            "StopDiscovery",
            "org.bluez.Adapter1",
            (),
            1000,
            (),
        );
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Could not stop bluetooth discovery\n",
                ErrorLevel::PartialBreakage
            );
        }
    }

    pub fn remove_device_pairing(&self, path: Path<'static>) -> Result<(), dbus::Error> {
        dbus_method!(
            BLUEZ_INTERFACE!(),
            self.current_adapter.clone(),
            "RemoveDevice",
            "org.bluez.Adapter1",
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
        ("org.bluez.Device1",),
        1000,
        (PropMap,),
    );
    if res.is_err() {
        ERROR!(
            "/tmp/reset_daemon_log",
            format!("Failed to get properties of bluetooth device: {}\n", path),
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
        "org.bluez.Adapter1",
        "Powered",
        enabled,
    );
    if res.is_err() {
        ERROR!(
            "/tmp/reset_daemon_log",
            format!(
                "Failed to set enabled mode on bluetooth adapter {} to: {}\n",
                path, enabled
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
        "org.bluez.Adapter1",
        "Discoverable",
        enabled,
    );
    if res.is_err() {
        ERROR!(
            "/tmp/reset_daemon_log",
            format!(
                "Failed to set discoverability mode on bluetooth adapter {} to: {}\n",
                path, enabled
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
        "org.bluez.Adapter1",
        "Pairable",
        enabled,
    );
    if res.is_err() {
        ERROR!(
            "/tmp/reset_daemon_log",
            format!(
                "Failed to set pairability mode on bluetooth adapter {} to: {}\n",
                path, enabled
            ),
            ErrorLevel::Recoverable
        );
        return false;
    }
    true
}

// command needed to understand anything about bluetooth
// dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
// /org/freedesktop/DBus org.freedesktop.DBus.ListNames | grep -v '":'
