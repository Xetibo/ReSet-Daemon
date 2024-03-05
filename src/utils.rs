use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    thread,
};

use crossbeam::channel::{unbounded, Receiver, Sender};
use dbus::{
    arg::{self, PropMap, RefArg, Variant},
    nonblock::SyncConnection,
    Path,
};
use once_cell::sync::{Lazy, OnceCell};
use re_set_lib::{
    audio::audio_structures::{Card, InputStream, OutputStream, Sink, Source},
    network::network_structures::Error,
    utils::get_system_dbus_property,
};
use tokio::task::JoinHandle;

use crate::{
    audio::audio_manager::PulseServer,
    bluetooth::bluetooth_manager::{BluetoothAgent, BluetoothInterface},
    network::network_manager::{get_wifi_devices, Device},
};

pub static CONSTANTS: OnceCell<ConstPaths> = OnceCell::new();

pub fn setup_constants(mode: Mode) -> ConstPaths {
    match mode {
        Mode::Test => setup_constants_test(),
        _ => setup_constants_regular(),
    }
}

pub fn setup_constants_test() -> ConstPaths {
    // TODO: add test options
    ConstPaths {
        dbus_path: "",
        wireless: "",
        bluetooth: "",
        audio: "",
        base: "",
    }
}

pub fn setup_constants_regular() -> ConstPaths {
    ConstPaths {
        dbus_path: "/org/Xetibo/ReSet/Daemon",
        wireless: "org.Xetibo.ReSet.Wireless",
        bluetooth: "org.Xetibo.ReSet.Bluetooth",
        audio: "org.Xetibo.ReSet.Audio",
        base: "org.Xetibo.ReSet.Daemon",
    }
}

pub enum Mode {
    Test,
    Debug,
    Release,
}

pub struct ConstPaths {
    pub dbus_path: &'static str,
    pub wireless: &'static str,
    pub bluetooth: &'static str,
    pub audio: &'static str,
    pub base: &'static str,
}

pub const AUDIO: &str = "org.Xetibo.ReSet.Audio";
pub const BASE: &str = "org.Xetibo.ReSet.Daemon";

pub type MaskedPropMap = HashMap<String, PropMap>;

pub type FullMaskedPropMap = HashMap<
    Path<'static>,
    HashMap<std::string::String, HashMap<std::string::String, dbus::arg::Variant<Box<dyn RefArg>>>>,
>;

pub enum AudioRequest {
    ListSources,
    GetDefaultSource,
    GetDefaultSourceName,
    SetSourceVolume(u32, u16, u32),
    SetSourceMute(u32, bool),
    SetDefaultSource(String),
    ListSinks,
    GetDefaultSink,
    GetDefaultSinkName,
    SetSinkVolume(u32, u16, u32),
    SetSinkMute(u32, bool),
    SetDefaultSink(String),
    ListInputStreams,
    SetSinkOfInputStream(u32, u32),
    SetInputStreamVolume(u32, u16, u32),
    SetInputStreamMute(u32, bool),
    ListOutputStreams,
    SetSourceOfOutputStream(u32, u32),
    SetOutputStreamVolume(u32, u16, u32),
    SetOutputStreamMute(u32, bool),
    ListCards,
    SetCardProfileOfDevice(u32, String),
    StopListener,
}

pub enum AudioResponse {
    DefaultSink(Sink),
    DefaultSource(Source),
    DefaultSinkName(String),
    DefaultSourceName(String),
    Sources(Vec<Source>),
    Sinks(Vec<Sink>),
    InputStreams(Vec<InputStream>),
    OutputStreams(Vec<OutputStream>),
    Cards(Vec<Card>),
    Error,
}

pub struct DaemonData {
    pub n_devices: Vec<Arc<RwLock<Device>>>,
    pub current_n_device: Arc<RwLock<Device>>,
    pub b_interface: BluetoothInterface,
    pub bluetooth_agent: BluetoothAgent,
    pub audio_sender: Arc<Sender<AudioRequest>>,
    pub audio_receiver: Arc<Receiver<AudioResponse>>,
    pub audio_listener_active: Arc<AtomicBool>,
    pub network_listener_active: Arc<AtomicBool>,
    pub network_stop_requested: Arc<AtomicBool>,
    pub bluetooth_listener_active: Arc<AtomicBool>,
    pub bluetooth_stop_requested: Arc<AtomicBool>,
    pub bluetooth_scan_active: Arc<AtomicBool>,
    pub clients: HashMap<String, usize>,
    pub connection: Arc<SyncConnection>,
    pub handle: JoinHandle<()>,
}

unsafe impl Send for DaemonData {}
unsafe impl Sync for DaemonData {}

impl DaemonData {
    pub fn create(
        handle: JoinHandle<()>,
        conn: Arc<SyncConnection>,
        features: &[&'static str],
    ) -> Result<Self, Error> {
        // TODO create check for pcs that don't offer wifi
        let mut n_devices = get_wifi_devices();
        let current_n_device = n_devices.pop().unwrap_or(Arc::new(RwLock::new(Device::new(
            Path::from("/"),
            String::from("empty"),
        ))));
        let b_interface_opt = BluetoothInterface::create(conn.clone());
        let b_interface: BluetoothInterface = if let Some(b_interface_opt) = b_interface_opt {
            b_interface_opt
        } else {
            BluetoothInterface::empty()
        };

        let (dbus_pulse_sender, pulse_receiver): (Sender<AudioRequest>, Receiver<AudioRequest>) =
            unbounded();
        let (pulse_sender, dbus_pulse_receiver): (Sender<AudioResponse>, Receiver<AudioResponse>) =
            unbounded();
        let audio_listener_active = Arc::new(AtomicBool::new(false));
        let audio_listener_ref = audio_listener_active.clone();
        let connection_ref = conn.clone();
        if features.contains(&"Audio") {
            thread::spawn(move || {
                let res = PulseServer::create(pulse_sender, pulse_receiver, connection_ref);
                if let Ok(mut res) = res {
                    audio_listener_ref.store(true, Ordering::SeqCst);
                    res.listen_to_messages();
                }
            });
        }

        Ok(DaemonData {
            n_devices,
            current_n_device,
            b_interface,
            bluetooth_agent: BluetoothAgent::new(),
            audio_sender: Arc::new(dbus_pulse_sender),
            audio_receiver: Arc::new(dbus_pulse_receiver),
            network_listener_active: Arc::new(AtomicBool::new(false)),
            network_stop_requested: Arc::new(AtomicBool::new(false)),
            audio_listener_active,
            bluetooth_listener_active: Arc::new(AtomicBool::new(false)),
            bluetooth_stop_requested: Arc::new(AtomicBool::new(false)),
            bluetooth_scan_active: Arc::new(AtomicBool::new(false)),
            connection: conn,
            handle,
            clients: HashMap::new(),
        })
    }
}

pub fn get_wifi_status() -> bool {
    let res = get_system_dbus_property::<(), bool>(
        "org.freedesktop.NetworkManager",
        Path::from("/org/freedesktop/NetworkManager"),
        "org.freedesktop.NetworkManager",
        "WirelessEnabled",
    );
    if res.is_err() {
        return false;
    }
    res.unwrap()
}

pub fn convert_bluetooth_map_bool(map_key: Option<&Variant<Box<dyn RefArg>>>) -> bool {
    if let Some(bonded_opt) = map_key {
        if let Some(bonded) = arg::cast::<bool>(&bonded_opt.0) {
            *bonded
        } else {
            false
        }
    } else {
        false
    }
}
