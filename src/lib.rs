mod api;
mod audio;
mod bluetooth;
mod network;
mod utils;

use std::{
    collections::HashMap,
    future::{self},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    thread,
};

use bluetooth::bluetooth::BluetoothAgent;
use dbus::{
    arg::PropMap, channel::MatchingReceiver, message::MatchRule, nonblock::SyncConnection, Message,
    Path,
};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection::{self};
use ReSet_Lib::{
    audio::audio::{Card, InputStream, OutputStream, Sink, Source},
    bluetooth::bluetooth::BluetoothDevice,
    network::network::{AccessPoint, Error},
    utils::{call_system_dbus_method, get_system_dbus_property},
};

use std::sync::mpsc::{self, Receiver, Sender};

use crate::{
    audio::audio::PulseServer,
    bluetooth::bluetooth::BluetoothInterface,
    network::network::{
        get_connection_settings, get_stored_connections, get_wifi_devices, list_connections,
        set_connection_settings, start_listener, stop_listener, Device,
    },
};

pub enum AudioRequest {
    ListSources,
    GetDefaultSource,
    SetSourceVolume(u32, u16, u32),
    SetSourceMute(u32, bool),
    SetDefaultSource(String),
    ListSinks,
    GetDefaultSink,
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
    Sources(Vec<Source>),
    Sinks(Vec<Sink>),
    InputStreams(Vec<InputStream>),
    OutputStreams(Vec<OutputStream>),
    Cards(Vec<Card>),
    BoolResponse(bool),
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
    pub connection: Arc<SyncConnection>,
}

unsafe impl Send for DaemonData {}
unsafe impl Sync for DaemonData {}

impl DaemonData {
    pub async fn create(conn: Arc<SyncConnection>) -> Result<Self, Error> {
        let mut n_devices = get_wifi_devices();
        if n_devices.is_empty() {
            return Err(Error {
                message: "Could not get any wifi devices",
            });
        }
        let current_n_device = n_devices.pop().unwrap();
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
            connection: conn,
        })
    }
}

pub async fn run_daemon() {
    let res = connection::new_session_sync();
    if res.is_err() {
        return;
    }
    let (resource, conn) = res.unwrap();
    let data = DaemonData::create(conn.clone()).await;
    if data.is_err() {
        return;
    }
    let data = data.unwrap();

    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    conn.request_name("org.Xetibo.ReSetDaemon", false, true, false)
        .await
        .unwrap();
    let mut cross = Crossroads::new();
    cross.set_async_support(Some((
        conn.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    let base = setup_base(&mut cross);
    let wireless_manager = setup_wireless_manager(&mut cross);
    let bluetooth_manager = setup_bluetooth_manager(&mut cross);
    let bluetooth_agent = setup_bluetooth_agent(&mut cross);
    let audio_manager = setup_audio_manager(&mut cross);

    cross.insert(
        "/org/xetibo/ReSetDaemon",
        &[
            base,
            wireless_manager,
            bluetooth_manager,
            bluetooth_agent,
            audio_manager,
        ],
        data,
    );

    conn.start_receive(
        MatchRule::new_method_call(),
        Box::new(move |msg, conn| {
            cross.handle_message(msg, conn).unwrap();
            true
        }),
    );

    future::pending::<()>().await;
    unreachable!()
}

fn setup_base(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    cross.register("org.Xetibo.ReSetDaemon", |c| {
        c.method("Check", (), ("result",), move |_, _, ()| Ok((true,)));
    })
}

fn setup_wireless_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register("org.Xetibo.ReSetWireless", |c| {
        c.signal::<(AccessPoint,), _>("AccessPointAdded", ("access_point",));
        c.signal::<(Path<'static>,), _>("AccessPointRemoved", ("path",));
        c.signal::<(AccessPoint,), _>("AccessPointChanged", ("access_point",));
        c.method(
            "ListAccessPoints",
            (),
            ("access_points",),
            move |_, d: &mut DaemonData, ()| {
                let access_points = d.current_n_device.read().unwrap().get_access_points();
                Ok((access_points,))
            },
        );
        c.method(
            "GetCurrentNetworkDevice",
            (),
            ("path", "name"),
            move |_, d: &mut DaemonData, ()| {
                let path = d.current_n_device.read().unwrap().dbus_path.clone();
                let name = get_system_dbus_property::<(), String>(
                    "org.freedesktop.NetworkManager",
                    path.clone(),
                    "org.freedesktop.NetworkManager.Device",
                    "Interface",
                );
                Ok((path, name.unwrap_or_else(|_| String::from(""))))
            },
        );
        c.method(
            "GetAllNetworkDevices",
            (),
            ("devices",),
            move |_, d: &mut DaemonData, ()| {
                let mut devices = Vec::new();
                let device_paths = get_wifi_devices();
                for device in device_paths {
                    let path = device.read().unwrap().dbus_path.clone();
                    let name = get_system_dbus_property::<(), String>(
                        "org.freedesktop.NetworkManager",
                        path.clone(),
                        "org.freedesktop.NetworkManager.Device",
                        "Interface",
                    );
                    devices.push((path, name.unwrap_or_else(|_| String::from(""))));
                }
                let path = d.current_n_device.read().unwrap().dbus_path.clone();
                let name = get_system_dbus_property::<(), String>(
                    "org.freedesktop.NetworkManager",
                    path.clone(),
                    "org.freedesktop.NetworkManager.Device",
                    "Interface",
                );
                devices.push((path, name.unwrap_or_else(|_| String::from(""))));
                Ok((devices,))
            },
        );
        c.method(
            "SetNetworkDevice",
            ("path",),
            ("result",),
            move |_, d: &mut DaemonData, (path,): (Path<'static>,)| {
                let mut res = false;
                let mut iter = 0;
                for device in d.n_devices.iter() {
                    if device.read().unwrap().dbus_path == path {
                        res = true;
                    }
                    iter += 1;
                }
                if res {
                    d.n_devices.push(d.current_n_device.clone());
                    d.current_n_device = d.n_devices.remove(iter);
                }
                Ok((res,))
            },
        );
        c.method(
            "ConnectToKnownAccessPoint",
            ("access_point",),
            ("result",),
            move |_, d: &mut DaemonData, (access_point,): (AccessPoint,)| {
                let res = d
                    .current_n_device
                    .write()
                    .unwrap()
                    .connect_to_access_point(access_point);
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
        c.method(
            "ConnectToNewAccessPoint",
            ("access_point", "password"),
            ("result",),
            move |_, d: &mut DaemonData, (access_point, password): (AccessPoint, String)| {
                let res = d
                    .current_n_device
                    .write()
                    .unwrap()
                    .add_and_connect_to_access_point(access_point, password);
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
        c.method(
            "DisconnectFromCurrentAccessPoint",
            (),
            ("result",),
            move |_, d: &mut DaemonData, ()| {
                let res = d
                    .current_n_device
                    .write()
                    .unwrap()
                    .disconnect_from_current();
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
        c.method("ListConnections", (), ("result",), move |_, _, ()| {
            let res = list_connections();
            Ok((res,))
        });
        c.method("ListStoredConnections", (), ("result",), move |_, _, ()| {
            let res = get_stored_connections();
            Ok((res,))
        });
        c.method(
            "GetConnectionSettings",
            ("path",),
            ("result",),
            move |_, _, (path,): (Path<'static>,)| {
                let res = get_connection_settings(path);
                if res.is_err() {
                    return Err(dbus::MethodErr::invalid_arg(
                        "Could not get settings for this connection.",
                    ));
                }
                Ok(res.unwrap())
            },
        );
        c.method(
            "SetConnectionSettings",
            ("path", "settings"),
            ("result",),
            move |_, _, (path, settings): (Path<'static>, HashMap<String, PropMap>)| {
                Ok((set_connection_settings(path, settings),))
            },
        );
        c.method(
            "DeleteConnection",
            ("path",),
            ("result",),
            move |_, _, (path,): (Path<'static>,)| {
                println!("called delete");
                let res = call_system_dbus_method::<(), ()>(
                    "org.freedesktop.NetworkManager",
                    path,
                    "Delete",
                    "org.freedesktop.NetworkManager.Settings.Connection",
                    (),
                    1000,
                );
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
        c.method_with_cr_async(
            "StartNetworkListener",
            (),
            ("result",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let path = data.current_n_device.read().unwrap().dbus_path.clone();
                let active_listener = data.network_listener_active.clone();
                let device = data.current_n_device.clone();
                let connection = data.connection.clone();
                thread::spawn(move || start_listener(connection, device, path, active_listener));
                async move { ctx.reply(Ok((true,))) }
            },
        );
        c.method(
            "StopNetworkListener",
            (),
            ("result",),
            move |_, data, ()| {
                let active_listener = data.network_listener_active.clone();
                stop_listener(active_listener);
                println!("stopped network listener");
                Ok((true,))
            },
        );
    });
    token
}

/// Bluetooth Manager API
/// The Bluetooth Manager handles searching for Bluetooth devices, as well as connecting and
/// disconnecting from Bluetooth devices.
fn setup_bluetooth_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register("org.Xetibo.ReSetBluetooth", |c| {
        c.signal::<(BluetoothDevice,), _>("BluetoothDeviceAdded", ("device",));
        c.signal::<(Path<'static>,), _>("BluetoothDeviceRemoved", ("path",));
        c.method_with_cr_async(
            "StartBluetoothSearch",
            ("duration",),
            (),
            move |mut ctx, cross, (duration,): (i32,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                data.b_interface.start_discovery(duration as u64);
                // let mut response = true;
                // if res.is_err() {
                //     response = false;
                // }
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method(
            "StopBluetoothSearch",
            (),
            ("result",),
            move |_, d: &mut DaemonData, ()| {
                let res = d.b_interface.stop_discovery();
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
        c.method(
            "ConnectToBluethoothDevice",
            ("device",),
            ("result",),
            move |_, d: &mut DaemonData, (device,): (Path<'static>,)| {
                let res = d.b_interface.connect_to(device);
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
        c.method(
            "PairWithBluetoothDevice",
            ("device",),
            ("result",),
            move |_, d: &mut DaemonData, (device,): (Path<'static>,)| {
                let res = d.b_interface.pair_with(device);
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
        c.method(
            "DisconnectFromBluetoothDevice",
            ("device",),
            ("result",),
            move |_, d: &mut DaemonData, (device,): (Path<'static>,)| {
                let res = d.b_interface.disconnect(device);
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
    });
    token
}

/// Audio Manager API
/// The audio manager handles all audio devices and their volume.
fn setup_audio_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    // TODO handle errors on the now not bool returning functions
    let token = cross.register("org.Xetibo.ReSetAudio", |c| {
        c.signal::<(Sink,), _>("SinkChanged", ("sink",));
        c.signal::<(Sink,), _>("SinkAdded", ("sink",));
        c.signal::<(u32,), _>("SinkRemoved", ("sink",));
        c.signal::<(Source,), _>("SourceChanged", ("source",));
        c.signal::<(Source,), _>("SourceAdded", ("source",));
        c.signal::<(u32,), _>("SourceRemoved", ("source",));
        c.signal::<(InputStream,), _>("InputStreamChanged", ("input_stream",));
        c.signal::<(InputStream,), _>("InputStreamAdded", ("input_stream",));
        c.signal::<(u32,), _>("InputStreamRemoved", ("input_stream",));
        c.signal::<(OutputStream,), _>("OutputStreamChanged", ("output_stream",));
        c.signal::<(OutputStream,), _>("OutputStreamAdded", ("output_stream",));
        c.signal::<(u32,), _>("OutputStreamRemoved", ("output_stream",));
        c.method_with_cr_async("StartAudioListener", (), (), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            if !data.audio_listener_active.load(Ordering::SeqCst) {
                let (dbus_pulse_sender, pulse_receiver): (
                    Sender<AudioRequest>,
                    Receiver<AudioRequest>,
                ) = mpsc::channel();
                let (pulse_sender, dbus_pulse_receiver): (
                    Sender<AudioResponse>,
                    Receiver<AudioResponse>,
                ) = mpsc::channel();

                data.audio_sender = Rc::new(dbus_pulse_sender);
                data.audio_receiver = Rc::new(dbus_pulse_receiver);
                let listener_active = data.audio_listener_active.clone();
                let connection = data.connection.clone();
                thread::spawn(move || {
                    let res = PulseServer::create(pulse_sender, pulse_receiver, connection);
                    if res.is_err() {
                        return;
                    }
                    listener_active.store(true, Ordering::SeqCst);
                    res.unwrap().listen_to_messages();
                });
            }
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async("StopAudioListener", (), (), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            if data.audio_listener_active.load(Ordering::SeqCst) {
                let _ = data.audio_sender.send(AudioRequest::StopListener);
            }
            data.audio_listener_active.store(false, Ordering::SeqCst);
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async(
            "GetDefaultSink",
            (),
            ("default_sink",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::GetDefaultSink);
                let response = data.audio_receiver.recv();
                let sink: Option<Sink> = if let Ok(response) = response {
                    match response {
                        AudioResponse::DefaultSink(s) => Some(s),
                        _ => None,
                    }
                } else {
                    None
                };
                let response: Result<(Sink,), dbus::MethodErr> = if let Some(sink) = sink {
                    Ok((sink,))
                } else {
                    Err(dbus::MethodErr::failed("Could not get default sink"))
                };
                async move { ctx.reply(response) }
            },
        );
        c.method_with_cr_async(
            "GetDefaultSource",
            (),
            ("default_source",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::GetDefaultSource);
                let response = data.audio_receiver.recv();
                let source: Option<Source> = if let Ok(response) = response {
                    match response {
                        AudioResponse::DefaultSource(s) => Some(s),
                        _ => None,
                    }
                } else {
                    None
                };
                let response: Result<(Source,), dbus::MethodErr> = if let Some(source) = source {
                    Ok((source,))
                } else {
                    Err(dbus::MethodErr::failed("Could not get default source"))
                };
                async move { ctx.reply(response) }
            },
        );
        c.method_with_cr_async("ListSinks", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let _ = data.audio_sender.send(AudioRequest::ListSinks);
            let response = data.audio_receiver.recv();
            let sinks: Vec<Sink> = if let Ok(response) = response {
                match response {
                    AudioResponse::Sinks(s) => s,
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            async move { ctx.reply(Ok((sinks,))) }
        });
        c.method_with_cr_async("ListSources", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let _ = data.audio_sender.send(AudioRequest::ListSources);
            let response = data.audio_receiver.recv();
            let sources: Vec<Source> = if let Ok(response) = response {
                match response {
                    AudioResponse::Sources(s) => s,
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            async move { ctx.reply(Ok((sources,))) }
        });
        c.method_with_cr_async(
            "SetSinkVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSinkVolume(index, channels, volume));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetSinkMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSinkMute(index, muted));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetSourceVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSourceVolume(index, channels, volume));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetSourceMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSourceMute(index, muted));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetDefaultSink",
            ("sink",),
            (),
            move |mut ctx, cross, (sink,): (String,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::SetDefaultSink(sink));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetDefaultSource",
            ("source",),
            (),
            move |mut ctx, cross, (source,): (String,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetDefaultSource(source));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ListInputStreams",
            (),
            ("input_streams",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::ListInputStreams);
                let response = data.audio_receiver.recv();
                let input_streams: Vec<InputStream> = if let Ok(response) = response {
                    match response {
                        AudioResponse::InputStreams(s) => s,
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                async move { ctx.reply(Ok((input_streams,))) }
            },
        );
        c.method_with_cr_async(
            "SetSinkOfInputStream",
            ("input_stream", "sink"),
            (),
            move |mut ctx, cross, (input_stream, sink): (u32, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSinkOfInputStream(input_stream, sink));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetInputStreamVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetInputStreamVolume(index, channels, volume));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetInputStreamMute",
            ("input_stream_index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetInputStreamMute(index, muted));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ListOutputStreams",
            (),
            ("output_streams",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::ListOutputStreams);
                let response = data.audio_receiver.recv();
                async move {
                    let output_streams: Vec<OutputStream> = if let Ok(response) = response {
                        match response {
                            AudioResponse::OutputStreams(s) => s,
                            _ => Vec::new(),
                        }
                    } else {
                        Vec::new()
                    };
                    ctx.reply(Ok((output_streams,)))
                }
            },
        );
        c.method_with_cr_async(
            "SetSourceOfOutputStream",
            ("input_stream", "source"),
            (),
            move |mut ctx, cross, (output_stream, source): (u32, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSourceOfOutputStream(output_stream, source));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetOutputStreamVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetOutputStreamVolume(index, channels, volume));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetOutputStreamMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetOutputStreamMute(index, muted));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async("ListCards", (), ("cards",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let _ = data.audio_sender.send(AudioRequest::ListCards);
            let response = data.audio_receiver.recv();
            async move {
                let cards: Vec<Card> = if let Ok(response) = response {
                    match response {
                        AudioResponse::Cards(s) => s,
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                ctx.reply(Ok((cards,)))
            }
        });
        c.method_with_cr_async(
            "SetCardProfileOfDevice",
            ("device_index", "profile_name"),
            (),
            move |mut ctx, cross, (device_index, profile_name): (u32, String)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::SetCardProfileOfDevice(
                    device_index,
                    profile_name,
                ));
                async move { ctx.reply(Ok(())) }
            },
        );
    });
    token
}

/// Bluetooth Agent API
/// The Bluetooth Agent is used to authorize connections and initiate pairing.
fn setup_bluetooth_agent(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register("org.Xetibo.ReSetBluetoothAgent", |c| {
        c.signal::<(), _>("PincodeRequested", ());
        c.signal::<(String,), _>("DisplayPinCode", ("code",));
        c.signal::<(), _>("PassKeyRequested", ());
        c.signal::<(u32, u16), _>("DisplayPassKey", ("passkey", "entered"));
        c.signal::<(), _>("PinCodeRequested", ());
        c.method(
            "RequestPinCode",
            ("device",),
            ("result",),
            move |ctx, d: &mut DaemonData, (_device,): (Path<'static>,)| {
                if d.bluetooth_agent.in_progress {
                    return Ok(("No pairing in progress.",));
                }
                let msg = Message::signal(
                    &Path::from("/org/Xetibo/ReSet"),
                    &"org.Xetibo.ReSetBluetoothAgent".into(),
                    &"PincodeRequested".into(),
                );
                ctx.push_msg(msg);
                Ok(("grengeng",))
                // TODO handle receive with a dynamic dbus function? does that even exist?
            },
        );
        c.method(
            "DisplayPinCode",
            ("device", "code"),
            (),
            move |ctx, _d: &mut DaemonData, (_device, code): (Path<'static>, String)| {
                let msg = Message::signal(
                    &Path::from("/org/Xetibo/ReSet"),
                    &"org.Xetibo.ReSetBluetoothAgent".into(),
                    &"DisplayPinCode".into(),
                )
                .append1(code);
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method(
            "RequestPassKey",
            ("device",),
            ("passkey",),
            move |ctx, _d: &mut DaemonData, (_device,): (Path<'static>,)| {
                let msg = Message::signal(
                    &Path::from("/org/Xetibo/ReSet"),
                    &"org.Xetibo.ReSetBluetoothAgent".into(),
                    &"RequestPassKey".into(),
                );
                ctx.push_msg(msg);
                Ok((0,))
            },
        );
        c.method(
            "DisplayPassKey",
            ("device", "passkey", "entered"),
            (),
            move |ctx,
                  _d: &mut DaemonData,
                  (_device, passkey, entered): (Path<'static>, u32, u16)| {
                let msg = Message::signal(
                    &Path::from("/org/Xetibo/ReSet"),
                    &"org.Xetibo.ReSetBluetoothAgent".into(),
                    &"DisplayPassKey".into(),
                )
                .append2(passkey, entered);
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method(
            "RequestConfirmation",
            ("device", "passkey"),
            (),
            move |ctx, _d: &mut DaemonData, (_device, passkey): (Path<'static>, u32)| {
                let msg = Message::signal(
                    &Path::from("/org/Xetibo/ReSet"),
                    &"org.Xetibo.ReSetBluetoothAgent".into(),
                    &"RequestConfirmation".into(),
                )
                .append1(passkey);
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method(
            "RequestAuthorization",
            ("device",),
            (),
            move |ctx, _d: &mut DaemonData, (_device,): (Path<'static>,)| {
                let msg = Message::signal(
                    &Path::from("/org/Xetibo/ReSet"),
                    &"org.Xetibo.ReSetBluetoothAgent".into(),
                    &"RequestAuthorization".into(),
                );
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method(
            "AuthorizeService",
            ("device", "uuid"),
            (),
            move |ctx, _d: &mut DaemonData, (_device, uuid): (Path<'static>, String)| {
                let msg = Message::signal(
                    &Path::from("/org/Xetibo/ReSet"),
                    &"org.Xetibo.ReSetBluetoothAgent".into(),
                    &"AuthorizeService".into(),
                )
                .append1(uuid);
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method("Cancel", (), (), move |_, d: &mut DaemonData, ()| {
            d.bluetooth_agent.in_progress = false;
            Ok(())
        });
        c.method("Release", (), (), move |_, d: &mut DaemonData, ()| {
            d.bluetooth_agent.in_progress = false;
            Ok(())
        });
    });

    token
}
