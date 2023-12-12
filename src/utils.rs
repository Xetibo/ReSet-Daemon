use std::{
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::AtomicBool,
        mpsc::{self, Receiver, Sender},
        Arc, RwLock,
    },
};

use dbus::{
    arg::{RefArg, Variant},
    nonblock::SyncConnection,
    Path,
};
use re_set_lib::{
    audio::audio_structures::{Card, InputStream, OutputStream, Sink, Source},
    network::network_structures::Error,
    utils::get_system_dbus_property,
};
use tokio::task::JoinHandle;

use crate::{
    bluetooth::bluetooth_manager::{BluetoothAgent, BluetoothInterface},
    network::network_manager::{get_wifi_devices, Device},
};

pub type MaskedPropMap = HashMap<String, HashMap<String, Variant<Box<dyn RefArg>>>>;

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
    pub audio_sender: Rc<Sender<AudioRequest>>,
    pub audio_receiver: Rc<Receiver<AudioResponse>>,
    pub audio_listener_active: Arc<AtomicBool>,
    pub network_listener_active: Arc<AtomicBool>,
    pub bluetooth_listener_active: Arc<AtomicBool>,
    pub bluetooth_scan_active: Arc<AtomicBool>,
    pub clients: HashMap<String, usize>,
    pub connection: Arc<SyncConnection>,
    pub handle: JoinHandle<()>,
}

unsafe impl Send for DaemonData {}
unsafe impl Sync for DaemonData {}

impl DaemonData {
    pub async fn create(handle: JoinHandle<()>, conn: Arc<SyncConnection>) -> Result<Self, Error> {
        let mut n_devices = get_wifi_devices();
        if n_devices.is_empty() {
            return Err(re_set_lib::network::network_structures::Error {
                message: "Could not get any wifi devices",
            });
        }
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

        let (dbus_pulse_sender, _): (Sender<AudioRequest>, Receiver<AudioRequest>) =
            mpsc::channel();
        let (_, dbus_pulse_receiver): (Sender<AudioResponse>, Receiver<AudioResponse>) =
            mpsc::channel();

        Ok(DaemonData {
            n_devices,
            current_n_device,
            b_interface,
            bluetooth_agent: BluetoothAgent::new(),
            audio_sender: Rc::new(dbus_pulse_sender),
            audio_receiver: Rc::new(dbus_pulse_receiver),
            network_listener_active: Arc::new(AtomicBool::new(false)),
            audio_listener_active: Arc::new(AtomicBool::new(false)),
            bluetooth_listener_active: Arc::new(AtomicBool::new(false)),
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
