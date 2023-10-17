mod bluez_signals;

use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};

use dbus::{
    arg::{self, Append, Arg, ArgType, RefArg, Variant},
    blocking::Connection,
    message::SignalArgs,
    Message, Path, Signature,
};
use dbus_crossroads::Context;

use crate::dbus::utils::set_system_dbus_property;

use self::bluez_signals::{InterfaceRemovedSignal, InterfacesAddedSignal};

use super::utils::call_system_dbus_method;

struct BConnection {}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    rssi: i16,
    name: String,
    adapter: Path<'static>,
    trusted: bool,
    bonded: bool,
    paired: bool,
    blocked: bool,
    address: String,
}

unsafe impl Send for BluetoothDevice {}
unsafe impl Sync for BluetoothDevice {}

impl Append for BluetoothDevice {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(&self.rssi);
            i.append(&self.name);
            i.append(&self.adapter);
            i.append(&self.trusted);
            i.append(&self.bonded);
            i.append(&self.paired);
            i.append(&self.blocked);
            i.append(&self.address);
        });
    }
}

impl Arg for BluetoothDevice {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(nsobbbbs)\0") }
    }
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

pub fn convert_device(
    map: &HashMap<String, HashMap<String, Variant<Box<dyn RefArg>>>>,
) -> Option<BluetoothDevice> {
    let map = map.get("org.bluez.Device1");
    if map.is_none() {
        return None;
    }
    let map = map.unwrap();
    let rssi: i16;
    let rssi_opt = map.get("RSSI");
    if rssi_opt.is_none() {
        rssi = -1;
    } else {
        rssi = *arg::cast::<i16>(&rssi_opt.unwrap().0).unwrap();
    }
    let name = arg::cast::<String>(&map.get("Alias").unwrap().0)
        .unwrap()
        .clone();
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
    Some(BluetoothDevice {
        rssi,
        name,
        adapter,
        trusted,
        bonded,
        paired,
        blocked,
        address,
    })
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
        let res = get_objects();
        if res.is_err() {
            return;
        }
        let (res,) = res.unwrap();
        for (path, map) in res.iter() {
            let device = convert_device(map);
            if device.is_some() {
                let device = device.unwrap();
                self.devices.insert(path.clone(), device);
            }
        }
    }

    pub fn start_discovery(&self, ctx: Arc<Mutex<Context>>) -> Result<(), dbus::Error> {
        let path = self.current_adapter.path.clone();
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy("org.bluez", path, Duration::from_millis(1000));
        let mr = InterfacesAddedSignal::match_rule(Some(&"org.bluez".into()), None).static_clone();
        let mrb =
            InterfaceRemovedSignal::match_rule(Some(&"org.bluez".into()), None).static_clone();
        let res = conn.add_match(mr, move |ir: InterfacesAddedSignal, _, _| {
            let device = convert_device(&ir.interfaces);
            if device.is_some() {
                let mut context = ctx.lock().unwrap();
                let device = device.unwrap();
                let signal = context.make_signal("BluetoothDeviceAdded", (ir.object, device));
                context.push_msg(signal);
            }
            true
        });
        if res.is_err() {
            return Err(dbus::Error::new_custom(
                "SignalMatchFailed",
                "Failed to match signal on bluez.",
            ));
        }
        let res = conn.add_match(mrb, |ir: InterfacesAddedSignal, _, _| {
            println!("Interfaces has been removed on path {}.\n\n\n", ir.object);
            // TODO
            // integrate this into daemon
            true
        });
        if res.is_err() {
            return Err(dbus::Error::new_custom(
                "SignalMatchFailed",
                "Failed to match signal on bluez.",
            ));
        }
        let res: Result<(), dbus::Error> =
            proxy.method_call("org.bluez.Adapter1", "StartDiscovery", ());
        let now = SystemTime::now();
        loop {
            let res = conn.process(Duration::from_millis(1000))?;
            if now.elapsed().unwrap() > Duration::from_millis(5000) {
                break;
            }
        }
        res
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
