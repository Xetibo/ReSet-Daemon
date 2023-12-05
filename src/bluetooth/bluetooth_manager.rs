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
    blocking::Connection,
    channel::Sender,
    message::SignalArgs,
    nonblock::SyncConnection,
    Error, Message, Path,
};
use dbus_tokio::connection;
use ReSet_Lib::{
    bluetooth::{
        bluetooth::{BluetoothAdapter, BluetoothDevice},
        bluetooth_signals::{BluetoothDeviceAdded, BluetoothDeviceRemoved},
    },
    signals::PropertiesChanged,
    utils::{call_system_dbus_method, set_system_dbus_property},
};

use crate::utils::{FullMaskedPropMap, MaskedPropMap};

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
    call_system_dbus_method::<
        (),
        (HashMap<Path<'static>, HashMap<String, HashMap<String, Variant<Box<dyn RefArg>>>>>,),
    >(
        interface,
        Path::from(path),
        "GetManagedObjects",
        "org.freedesktop.DBus.ObjectManager",
        (),
        1000,
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
    let alias = arg::cast::<String>(&map.get("Alias").unwrap().0)
        .unwrap()
        .clone();
    let adapter = arg::cast::<Path<'static>>(&map.get("Adapter").unwrap().0)
        .unwrap()
        .clone();
    let trusted = *arg::cast::<bool>(&map.get("Trusted").unwrap().0).unwrap();
    let blocked = *arg::cast::<bool>(&map.get("Blocked").unwrap().0).unwrap();
    let bonded = *arg::cast::<bool>(&map.get("Bonded").unwrap().0).unwrap();
    let paired = *arg::cast::<bool>(&map.get("Paired").unwrap().0).unwrap();
    let connected_opt: Option<&bool> = prop_cast(map, "Connected");
    let icon_opt: Option<&String> = prop_cast(map, "Icon");
    let icon = if let Some(icon_opt) = icon_opt {
        icon_opt.clone()
    } else {
        String::from("")
    };
    let address = arg::cast::<String>(&map.get("Address").unwrap().0)
        .unwrap()
        .clone();
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
    let res = call_system_dbus_method::<(&str,), (PropMap,)>(
        "org.bluez",
        path.clone(),
        "GetAll",
        "org.freedesktop.DBus.Properties",
        ("org.bluez.Adapter1",),
        1000,
    );
    let map = if let Ok(res) = res {
        res.0
    } else {
        PropMap::new()
    };
    dbg!(&map);
    adapter_from_map(path, &map)
}

pub fn get_connections() -> Vec<ReSet_Lib::bluetooth::bluetooth::BluetoothDevice> {
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
        let res = get_objects("org.bluez", "/");
        if res.is_err() {
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
        let mut interface = Self {
            adapters,
            current_adapter,
            devices: HashMap::new(),
            enabled: false,
            registered: false,
            in_discovery: Arc::new(AtomicBool::new(false)),
            connection: conn,
        };
        interface.set_bluetooth(true);
        Some(interface)
    }

    pub fn start_bluetooth_listener(&self, active_listener: Arc<AtomicBool>) {
        let path = self.current_adapter.clone();
        // let path_loop = self.current_adapter.clone();
        let added_ref = self.connection.clone();
        let removed_ref = self.connection.clone();
        let changed_ref = self.connection.clone();
        let discovery_active = self.in_discovery.clone();
        thread::spawn(move || {
            if active_listener.load(Ordering::SeqCst) {
                discovery_active.store(true, Ordering::SeqCst);
                return Ok(());
            }
            let conn = Connection::new_system().unwrap();
            let proxy = conn.with_proxy("org.bluez", path, Duration::from_millis(1000));
            let mr =
                BluetoothDeviceAdded::match_rule(Some(&"org.bluez".into()), None).static_clone();
            let mrb =
                BluetoothDeviceRemoved::match_rule(Some(&"org.bluez".into()), None).static_clone();
            let bluetooth_device_changed =
                PropertiesChanged::match_rule(Some(&"org.bluez".into()), None).static_clone();
            let res = conn.add_match(mr, move |ir: BluetoothDeviceAdded, _, _| {
                let device = convert_device(&ir.object, &ir.interfaces);
                if let Some(device) = device {
                    let msg = Message::signal(
                        &Path::from("/org/Xetibo/ReSetDaemon"),
                        &"org.Xetibo.ReSetBluetooth".into(),
                        &"BluetoothDeviceAdded".into(),
                    )
                    .append1(device);
                    let _ = added_ref.send(msg);
                }
                true
            });
            if res.is_err() {
                return Err(dbus::Error::new_custom(
                    "SignalMatchFailed",
                    "Failed to match signal on bluez.",
                ));
            }
            let res = conn.add_match(mrb, move |ir: BluetoothDeviceRemoved, _, _| {
                let msg = Message::signal(
                    &Path::from("/org/Xetibo/ReSetDaemon"),
                    &"org.Xetibo.ReSetBluetooth".into(),
                    &"BluetoothDeviceRemoved".into(),
                )
                .append1(ir.object);
                let _ = removed_ref.send(msg);
                true
            });
            if res.is_err() {
                return Err(dbus::Error::new_custom(
                    "SignalMatchFailed",
                    "Failed to match signal on bluez.",
                ));
            }
            let res = conn.add_match(
                bluetooth_device_changed,
                move |_: PropertiesChanged, _, msg| {
                    if let Some(path) = msg.path() {
                        let path = Path::from(path.to_string());
                        let map = get_bluetooth_device_properties(&path);
                        let device_opt = bluetooth_device_from_map(&path, &map);

                        if let Some(device) = device_opt {
                            let msg = Message::signal(
                                &Path::from("/org/Xetibo/ReSetDaemon"),
                                &"org.Xetibo.ReSetBluetooth".into(),
                                &"BluetoothDeviceChanged".into(),
                            )
                            .append1(device);
                            let _ = changed_ref.send(msg);
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
                return Err(dbus::Error::new_custom(
                    "SignalMatchFailed",
                    "Failed to match signal on bluez.",
                ));
            }
            let res: Result<(), dbus::Error> =
                proxy.method_call("org.bluez.Adapter1", "StartDiscovery", ());
            active_listener.store(true, Ordering::SeqCst);
            let mut is_discovery = true;
            loop {
                let _ = conn.process(Duration::from_millis(1000))?;
                if !active_listener.load(Ordering::SeqCst) {
                    discovery_active.store(false, Ordering::SeqCst);
                    let _: Result<(), dbus::Error> =
                        proxy.method_call("org.bluez.Adapter1", "StopDiscovery", ());
                    break;
                } else if !is_discovery && discovery_active.load(Ordering::SeqCst) {
                    is_discovery = true;
                    let _: Result<(), dbus::Error> =
                        proxy.method_call("org.bluez.Adapter1", "StartDiscovery", ());
                }
            }
            res
        });
    }

    pub fn connect_to(&self, device: Path<'static>) {
        thread::spawn(move || {
            let _ = call_system_dbus_method::<(), ()>(
                "org.bluez",
                device,
                "Connect",
                "org.bluez.Device1",
                (),
                10000,
            );
        });
    }

    pub fn pair_with(&mut self, device: Path<'static>) {
        if !self.registered {
            self.register_agent();
        }
        thread::spawn(move || {
            // TODO handle this error later on? If so how?
            let _ = call_system_dbus_method::<(), ()>(
                "org.bluez",
                device,
                "Pair",
                "org.bluez.Device1",
                (),
                10000,
            );
        });
    }

    pub fn disconnect(&self, device: Path<'static>) -> Result<(), dbus::Error> {
        call_system_dbus_method::<(), ()>(
            "org.bluez",
            device,
            "Disconnect",
            "org.bluez.Device1",
            (),
            1000,
        )
    }

    pub fn set_bluetooth(&mut self, value: bool) {
        let res = set_system_dbus_property(
            "org.bluez",
            self.current_adapter.clone(),
            "org.bluez.Adapter1",
            "Powered",
            value,
        );
        if res.is_err() {
            self.enabled = false;
            return;
        }
        self.enabled = value;
    }

    pub fn register_agent(&mut self) -> bool {
        if self.registered {
            return false;
        }
        let res = call_system_dbus_method::<(Path<'static>, &'static str), ()>(
            "org.bluez",
            Path::from("/org/bluez"),
            "RegisterAgent",
            "org.bluez.AgentManager1",
            (Path::from("/org/Xetibo/ReSetDaemon"), "DisplayYesNo"),
            1000,
        );
        if res.is_err() {
            return false;
        }
        self.registered = true;
        true
    }

    pub fn unregister_agent(&mut self) -> bool {
        if !self.registered {
            return false;
        }
        let res = call_system_dbus_method::<(Path<'static>,), ()>(
            "org.bluez",
            Path::from("/org/bluez"),
            "UnregisterAgent",
            "org.bluez.AgentManager1",
            (Path::from("/org/Xetibo/ReSetDaemon"),),
            1000,
        );
        if res.is_err() {
            return false;
        }
        self.registered = false;
        true
    }

    pub fn start_bluetooth_discovery(&self) -> Result<(), dbus::Error> {
        if self.in_discovery.load(Ordering::SeqCst) {
            return Ok(());
        }
        call_system_dbus_method::<(), ()>(
            "org.bluez",
            self.current_adapter.clone(),
            "StartDiscovery",
            "org.bluez.Adapter1",
            (),
            1000,
        )
    }

    pub fn stop_bluetooth_discovery(&self) -> Result<(), dbus::Error> {
        if !self.in_discovery.load(Ordering::SeqCst) {
            return Ok(());
        }
        call_system_dbus_method::<(), ()>(
            "org.bluez",
            self.current_adapter.clone(),
            "StopDiscovery",
            "org.bluez.Adapter1",
            (),
            1000,
        )
    }

    pub fn remove_device_pairing(&self, path: Path<'static>) -> Result<(), dbus::Error> {
        call_system_dbus_method::<(Path<'static>,), ()>(
            "org.bluez",
            self.current_adapter.clone(),
            "RemoveDevice",
            "org.bluez.Adapter1",
            (path,),
            1000,
        )
    }
}

fn get_bluetooth_device_properties(path: &Path<'static>) -> PropMap {
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy("org.bluez", path, Duration::from_millis(1000));
    let res: Result<(PropMap,), Error> = proxy.method_call(
        "org.freedesktop.DBus.Properties",
        "GetAll",
        ("org.bluez.Device1",),
    );
    if res.is_err() {
        return PropMap::new();
    }
    res.unwrap().0
}

pub fn set_adapter_enabled(path: Path<'static>, enabled: bool) -> bool {
    let res = set_system_dbus_property("org.bluez", path, "org.bluez.Adapter1", "Powered", enabled);
    if res.is_err() {
        return false;
    }
    true
}

pub fn set_adapter_discoverable(path: Path<'static>, enabled: bool) -> bool {
    dbg!(path.clone());
    let res = set_system_dbus_property(
        "org.bluez",
        path,
        "org.bluez.Adapter1",
        "Discoverable",
        enabled,
    );
    if res.is_err() {
        return false;
    }
    true
}

pub fn set_adapter_pairable(path: Path<'static>, enabled: bool) -> bool {
    dbg!(path.clone());
    let res =
        set_system_dbus_property("org.bluez", path, "org.bluez.Adapter1", "Pairable", enabled);
    if res.is_err() {
        return false;
    }
    true
}

// command needed to understand anything about bluetooth
// dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
// /org/freedesktop/DBus org.freedesktop.DBus.ListNames | grep -v '":'
