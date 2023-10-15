mod bluez_signals;

use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use dbus::{
    arg::{self, RefArg, Variant},
    blocking::{Connection, MakeSignal},
    Message, Path,
};

use crate::dbus::utils::set_system_dbus_property;

use self::bluez_signals::InterfacesAddedSignal;

use super::utils::call_system_dbus_method;

struct BConnection {}

#[derive(Debug, Clone)]
struct BluetoothDevice {
    rssi: i16,
    name: String,
    adapter: Path<'static>,
    trusted: bool,
    bonded: bool,
    paired: bool,
    blocked: bool,
    address: String,
}

#[derive(Debug, Clone)]
struct BluetoothAdapter {
    path: Path<'static>,
}

#[derive(Debug, Clone)]
pub struct BluetoothInterface {
    adapters: Vec<BluetoothAdapter>,
    current_adapter: BluetoothAdapter,
    devices: HashMap<Path<'static>, BluetoothDevice>,
    enabled: bool,
    real: bool,
}

fn get_objects() -> Result<
    (
        HashMap<
            Path<'static>,
            HashMap<
                std::string::String,
                HashMap<std::string::String, dbus::arg::Variant<Box<dyn RefArg>>>,
            >,
        >,
    ),
    dbus::Error,
> {
    let res = call_system_dbus_method::<
        (),
        (HashMap<Path<'static>, HashMap<String, HashMap<String, Variant<Box<dyn RefArg>>>>>,),
    >(
        "org.bluez",
        Path::from("/"),
        "GetManagedObjects",
        "org.freedesktop.DBus.ObjectManager",
        (),
        1000,
    );
    res
}

impl BluetoothInterface {
    pub fn empty() -> Self {
        Self {
            adapters: Vec::new(),
            current_adapter: BluetoothAdapter {
                path: Path::from("/"),
            },
            devices: HashMap::new(),
            enabled: false,
            real: false,
        }
    }
    pub fn create() -> Option<Self> {
        let mut adapters = Vec::new();
        let res = get_objects();
        if res.is_err() {
            return None;
        }
        let (res,) = res.unwrap();
        for (path, map) in res.iter() {
            let map = map.get("org.bluez.Adapter1");
            if map.is_none() {
                continue;
            }
            let map = map.unwrap();
            adapters.push(BluetoothAdapter { path: path.clone() });
        }
        if adapters.len() < 1 {
            return None;
        }
        let current_adapter = adapters.pop().unwrap();
        let mut interface = Self {
            adapters,
            current_adapter,
            devices: HashMap::new(),
            enabled: false,
            real: true,
        };
        let res = interface.set_bluetooth(true);
        Some(interface)
    }

    pub fn get_connections(&mut self) {
        let res = self.start_discovery();
        if res.is_err() {
            return;
        }
        let res = get_objects();
        if res.is_err() {
            return;
        }
        let (res,) = res.unwrap();
        for (path, map) in res.iter() {
            let map = map.get("org.bluez.Device1");
            if map.is_none() {
                continue;
            }
            let map = map.unwrap();
            let rssi = *arg::cast::<i16>(&map.get("RSSI").unwrap().0).unwrap();
            let name = arg::cast::<String>(&map.get("Alias").unwrap().0)
                .unwrap()
                .clone();
            let path = arg::cast::<Path<'static>>(path).unwrap().clone();
            let adapter = arg::cast::<Path<'static>>(&map.get("Adapter").unwrap().0)
                .unwrap()
                .clone();
            let trusted = *arg::cast::<bool>(&map.get("Trusted").unwrap().0).unwrap();
            let blocked = *arg::cast::<bool>(&map.get("Blocked").unwrap().0).unwrap();
            let bonded = *arg::cast::<bool>(&map.get("Bonded").unwrap().0).unwrap();
            let paired = *arg::cast::<bool>(&map.get("Paired").unwrap().0).unwrap();
            let address = arg::cast::<String>(&map.get("Address").unwrap().0)
                .unwrap()
                .clone();
            self.devices.insert(
                path,
                BluetoothDevice {
                    rssi,
                    name,
                    adapter,
                    trusted,
                    bonded,
                    paired,
                    blocked,
                    address,
                },
            );
        }
    }

    pub fn start_discovery(&self) -> Result<(), dbus::Error> {
        // call_system_dbus_method::<(), ()>(
        //     "org.bluez",
        //     self.current_adapter.path.clone(),
        //     "StartDiscovery",
        //     "org.bluez.Adapter1",
        //     (),
        //     10000,
        // )
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy(
            "org.bluez",
            self.current_adapter.path.clone(),
            Duration::from_millis(1000),
        );
        let _id = proxy.match_signal(|sig: InterfacesAddedSignal, _: &Connection, _: &Message| {
            println!("{}", sig.object);
            true
        });
        let res: Result<(), dbus::Error> =
            proxy.method_call("org.bluez.Adapter1", "StartDiscovery", ());
        let now = SystemTime::now();
        loop {
            conn.process(Duration::from_millis(1000))?;
            if now.elapsed().unwrap() > Duration::from_millis(5000) {
                break;
            }
        }
        Ok(())
    }

    pub fn stop_discovery(&self) -> Result<(), dbus::Error> {
        call_system_dbus_method::<(), ()>(
            "org.bluez",
            self.current_adapter.path.clone(),
            "StopDiscovery",
            "org.bluez",
            (),
            1000,
        )
    }

    pub fn connect_to() {
        todo!()
    }

    pub fn disconnect() {
        todo!()
    }

    pub fn set_bluetooth(&mut self, value: bool) {
        let res = set_system_dbus_property(
            "org.bluez",
            self.current_adapter.path.clone(),
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
}

// command needed to understand anything about bluetooth
// dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply \
// /org/freedesktop/DBus org.freedesktop.DBus.ListNames | grep -v '":'
