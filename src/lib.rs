mod audio;
mod bluetooth;
mod network;

use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::HashMap,
    future::{self},
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
};

use dbus::{arg::PropMap, channel::MatchingReceiver, message::MatchRule, Path};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection::{self};
use tokio;
use ReSet_Lib::{
    audio::audio::{InputStream, OutputStream, Sink, Source},
    bluetooth::bluetooth::BluetoothDevice,
    network::network::{AccessPoint, Error},
    utils::{call_system_dbus_method, get_system_dbus_property},
};

// use crate::network::network::{
// get_connection_settings, list_connections, set_connection_settings, start_listener,
// stop_listener,
// };

// use bluetooth::bluetooth::BluetoothInterface;
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
    SetSourceVolume(Source),
    SetSourceMute(Source),
    SetDefaultSource(Source),
    ListSinks,
    GetDefaultSink,
    SetSinkVolume(Sink),
    SetSinkMute(Sink),
    SetDefaultSink(Sink),
    ListInputStreams,
    SetSinkOfInputStream(InputStream, Sink),
    SetInputStreamVolume(InputStream),
    SetInputStreamMute(InputStream),
    ListOutputStreams,
    SetSourceOfOutputStream(OutputStream, Source),
    SetOutputStreamVolume(OutputStream),
    SetOutputStreamMute(OutputStream),
}

pub enum AudioResponse {
    DefaultSink(Sink),
    DefaultSource(Source),
    Sources(Vec<Source>),
    Sinks(Vec<Sink>),
    InputStreams(Vec<InputStream>),
    OutputStreams(Vec<OutputStream>),
    BoolResponse(bool),
}

pub struct DaemonData {
    pub n_devices: Vec<Device>,
    pub current_n_device: Device,
    pub b_interface: BluetoothInterface,
    pub audio_sender: Sender<AudioRequest>,
    pub audio_receiver: Receiver<AudioResponse>,
    pub active_listener: Arc<AtomicBool>,
}

unsafe impl Send for DaemonData {}
unsafe impl Sync for DaemonData {}

impl DaemonData {
    pub async fn create() -> Result<Self, Error> {
        let mut n_devices = get_wifi_devices();
        if n_devices.len() < 1 {
            return Err(Error {
                message: "Could not get any wifi devices",
            });
        }
        let current_n_device = n_devices.pop().unwrap();
        let b_interface_opt = BluetoothInterface::create();
        let b_interface: BluetoothInterface;
        if b_interface_opt.is_none() {
            b_interface = BluetoothInterface::empty();
        } else {
            b_interface = b_interface_opt.unwrap();
        }

        let (dbus_pulse_sender, pulse_receiver): (Sender<AudioRequest>, Receiver<AudioRequest>) =
            mpsc::channel();
        let (pulse_sender, dbus_pulse_receiver): (Sender<AudioResponse>, Receiver<AudioResponse>) =
            mpsc::channel();

        thread::spawn(move || {
            let res = PulseServer::create(pulse_sender, pulse_receiver);
            if res.is_err() {
                return;
            }
            res.unwrap().listen_to_messages();
        });
        Ok(DaemonData {
            n_devices,
            current_n_device,
            b_interface,
            audio_sender: dbus_pulse_sender,
            audio_receiver: dbus_pulse_receiver,
            active_listener: Arc::new(AtomicBool::new(false)),
        })
    }
}

pub async fn run_daemon() {
    let data = DaemonData::create().await;
    if data.is_err() {
        return;
    }
    let data = data.unwrap();
    let res = connection::new_session_sync();
    if res.is_err() {
        return;
    }
    let (resource, conn) = res.unwrap();

    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    conn.request_name("org.xetibo.ReSet", false, true, false)
        .await
        .unwrap();
    let mut cross = Crossroads::new();
    cross.set_async_support(Some((
        conn.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    let token = cross.register("org.xetibo.ReSet", |c| {
        let bluetooth_device_added = c
            .signal::<(BluetoothDevice,), _>("BluetoothDeviceAdded", ("device",))
            .msg_fn();
        let bluetooth_device_removed = c
            .signal::<(Path<'static>,), _>("BluetoothDeviceRemoved", ("path",))
            .msg_fn();
        let access_point_added = c
            .signal::<(AccessPoint,), _>("AccessPointAdded", ("access_point",))
            .msg_fn();
        let access_point_removed = c
            .signal::<(AccessPoint,), _>("AccessPointRemoved", ("access_point",))
            .msg_fn();
        let access_point_changed = c
            .signal::<(PropMap,), _>("AccessPointChanged", ("map",))
            .msg_fn();
        let sink_added = c.signal::<(Sink,), _>("SinkAdded", ("sink",)).msg_fn();
        let sink_removed = c.signal::<(Sink,), _>("SinkRemoved", ("sink",)).msg_fn();
        let sink_changed = c.signal::<(Sink,), _>("SinkChanged", ("sink",)).msg_fn();
        let source_added = c
            .signal::<(Source,), _>("SourceAdded", ("source",))
            .msg_fn();
        let source_removed = c
            .signal::<(Source,), _>("SourceRemoved", ("source",))
            .msg_fn();
        let source_changed = c
            .signal::<(Source,), _>("SourceChanged", ("source",))
            .msg_fn();
        let input_stream_added = c
            .signal::<(InputStream,), _>("InputStreamAdded", ("input_stream",))
            .msg_fn();
        let input_stream_removed = c
            .signal::<(InputStream,), _>("InputStreamRemoved", ("input_stream",))
            .msg_fn();
        let input_stream_changed = c
            .signal::<(InputStream,), _>("InputStreamChanged", ("input_stream",))
            .msg_fn();
        let output_stream_added = c
            .signal::<(OutputStream,), _>("OutputStreamAdded", ("output_stream",))
            .msg_fn();
        let output_stream_removed = c
            .signal::<(OutputStream,), _>("OutputStreamRemoved", ("output_stream",))
            .msg_fn();
        let output_stream_changed = c
            .signal::<(OutputStream,), _>("OutputStreamChanged", ("output_stream",))
            .msg_fn();
        c.method("Check", (), ("result",), move |_, _, ()| Ok((true,)));
        c.method(
            "ListAccessPoints",
            (),
            ("access_points",),
            move |_, d: &mut DaemonData, ()| {
                let access_points = d.current_n_device.get_access_points();
                Ok((access_points,))
            },
        );
        c.method(
            "GetCurrentNetworkDevice",
            (),
            ("path", "name"),
            move |_, d: &mut DaemonData, ()| {
                let name = get_system_dbus_property::<(), String>(
                    "org.freedesktop.NetworkManager",
                    d.current_n_device.dbus_path.clone(),
                    "org.freedesktop.NetworkManager.Device",
                    "Interface",
                );
                Ok((
                    d.current_n_device.dbus_path.clone(),
                    name.unwrap_or_else(|_| String::from("")),
                ))
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
                    let name = get_system_dbus_property::<(), String>(
                        "org.freedesktop.NetworkManager",
                        device.dbus_path.clone(),
                        "org.freedesktop.NetworkManager.Device",
                        "Interface",
                    );
                    devices.push((device.dbus_path, name.unwrap_or_else(|_| String::from(""))));
                }
                let name = get_system_dbus_property::<(), String>(
                    "org.freedesktop.NetworkManager",
                    d.current_n_device.dbus_path.clone(),
                    "org.freedesktop.NetworkManager.Device",
                    "Interface",
                );
                devices.push((
                    d.current_n_device.dbus_path.clone(),
                    name.unwrap_or_else(|_| String::from("")),
                ));
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
                    if device.dbus_path == path {
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
                let res = d.current_n_device.connect_to_access_point(access_point);
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
                let res = d.current_n_device.disconnect_from_current();
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
                let path = data.current_n_device.dbus_path.clone();
                let active_listener = data.active_listener.clone();
                let access_points = data.current_n_device.get_access_points();
                thread::spawn(move || start_listener(access_points, path, active_listener));
                async move { ctx.reply(Ok((true,))) }
            },
        );
        c.method(
            "StopNetworkListener",
            (),
            ("result",),
            move |_, data, ()| {
                let active_listener = data.active_listener.clone();
                stop_listener(active_listener);
                println!("stopped network listener");
                Ok((true,))
            },
        );
        c.method_with_cr_async(
            "StartBluetoothSearch",
            ("duration",),
            ("result",),
            move |ctx, cross, (duration,): (i32,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let ctx_ref = Arc::new(Mutex::new(ctx));
                let res = data.b_interface.start_discovery(duration as u64);
                let mut response = true;
                if res.is_err() {
                    response = false;
                }
                let mut ctx = Arc::try_unwrap(ctx_ref).unwrap().into_inner().unwrap();
                async move { ctx.reply(Ok((response,))) }
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
        c.method_with_cr_async(
            "GetDefaultSink",
            (),
            ("default_sink",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sink: Option<Sink>;
                let _ = data.audio_sender.send(AudioRequest::GetDefaultSink);
                let response = data.audio_receiver.recv();
                if response.is_ok() {
                    sink = match response.unwrap() {
                        AudioResponse::DefaultSink(s) => Some(s),
                        _ => None,
                    }
                } else {
                    sink = None;
                }
                let response: Result<(Sink,), dbus::MethodErr>;
                if sink.is_none() {
                    response = Err(dbus::MethodErr::failed("Could not get default sink"));
                } else {
                    response = Ok((sink.unwrap(),));
                }
                async move { ctx.reply(response) }
            },
        );
        c.method_with_cr_async(
            "GetDefaultSource",
            (),
            ("default_source",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let source: Option<Source>;
                let _ = data.audio_sender.send(AudioRequest::GetDefaultSource);
                let response = data.audio_receiver.recv();
                if response.is_ok() {
                    source = match response.unwrap() {
                        AudioResponse::DefaultSource(s) => Some(s),
                        _ => None,
                    }
                } else {
                    source = None;
                }
                let response: Result<(Source,), dbus::MethodErr>;
                if source.is_none() {
                    response = Err(dbus::MethodErr::failed("Could not get default sink"));
                } else {
                    response = Ok((source.unwrap(),));
                }
                async move { ctx.reply(response) }
            },
        );
        c.method_with_cr_async("ListSinks", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let sinks: Vec<Sink>;
            let _ = data.audio_sender.send(AudioRequest::ListSinks);
            let response = data.audio_receiver.recv();
            if response.is_ok() {
                sinks = match response.unwrap() {
                    AudioResponse::Sinks(s) => s,
                    _ => Vec::new(),
                }
            } else {
                sinks = Vec::new();
            }
            async move { ctx.reply(Ok((sinks,))) }
        });
        c.method_with_cr_async("ListSources", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let sources: Vec<Source>;
            let _ = data.audio_sender.send(AudioRequest::ListSources);
            let response = data.audio_receiver.recv();
            if response.is_ok() {
                sources = match response.unwrap() {
                    AudioResponse::Sources(s) => s,
                    _ => Vec::new(),
                }
            } else {
                sources = Vec::new();
            }
            async move { ctx.reply(Ok((sources,))) }
        });
        c.method_with_cr_async(
            "SetSinkVolume",
            ("sink",),
            ("result",),
            move |mut ctx, cross, (sink,): (Sink,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::SetSinkVolume(sink));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetSinkMute",
            ("sink",),
            ("result",),
            move |mut ctx, cross, (sink,): (Sink,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::SetSinkMute(sink));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetSourceVolume",
            ("source",),
            ("result",),
            move |mut ctx, cross, (source,): (Source,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSourceVolume(source));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetSourceMute",
            ("source",),
            ("result",),
            move |mut ctx, cross, (source,): (Source,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::SetSourceMute(source));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetDefaultSink",
            ("sink",),
            ("result",),
            move |mut ctx, cross, (sink,): (Sink,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::SetDefaultSink(sink));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetDefaultSource",
            ("source",),
            ("result",),
            move |mut ctx, cross, (source,): (Source,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetDefaultSource(source));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "ListInputStreams",
            (),
            ("input_streams",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let input_streams: Vec<InputStream>;
                let _ = data.audio_sender.send(AudioRequest::ListInputStreams);
                let response = data.audio_receiver.recv();
                if response.is_ok() {
                    input_streams = match response.unwrap() {
                        AudioResponse::InputStreams(s) => s,
                        _ => Vec::new(),
                    }
                } else {
                    input_streams = Vec::new();
                }
                async move { ctx.reply(Ok((input_streams,))) }
            },
        );
        c.method_with_cr_async(
            "SetSinkofInputStream",
            ("input_stream", "sink"),
            ("result",),
            move |mut ctx, cross, (input_stream, sink): (InputStream, Sink)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSinkOfInputStream(input_stream, sink));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetInputStreamVolume",
            ("sink",),
            ("result",),
            move |mut ctx, cross, (input_stream,): (InputStream,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetInputStreamVolume(input_stream));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetInputStreamMute",
            ("sink",),
            ("result",),
            move |mut ctx, cross, (input_stream,): (InputStream,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetInputStreamMute(input_stream));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "ListOutputStreams",
            (),
            ("output_streams",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let output_streams: Vec<OutputStream>;
                let _ = data.audio_sender.send(AudioRequest::ListOutputStreams);
                let response = data.audio_receiver.recv();
                if response.is_ok() {
                    output_streams = match response.unwrap() {
                        AudioResponse::OutputStreams(s) => s,
                        _ => Vec::new(),
                    }
                } else {
                    output_streams = Vec::new();
                }
                async move { ctx.reply(Ok((output_streams,))) }
            },
        );
        c.method_with_cr_async(
            "SetSourceOfOutputStream",
            ("input_stream", "source"),
            ("result",),
            move |mut ctx, cross, (output_stream, source): (OutputStream, Source)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSourceOfOutputStream(output_stream, source));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetOutputStreamVolume",
            ("sink",),
            ("result",),
            move |mut ctx, cross, (output_stream,): (OutputStream,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetOutputStreamVolume(output_stream));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "SetOutputStreamMute",
            ("sink",),
            ("result",),
            move |mut ctx, cross, (output_stream,): (OutputStream,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetOutputStreamMute(output_stream));
                let result: bool;
                let res = data.audio_receiver.recv();
                if res.is_err() {
                    result = false;
                } else {
                    result = match res.unwrap() {
                        AudioResponse::BoolResponse(b) => b,
                        _ => false,
                    };
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        // these are for the listener, other synchroniztion methods seem to not work....
        c.method_with_cr_async(
            "AddAccessPointEvent",
            ("access_point",),
            (),
            move |mut ctx, _, access_point: (AccessPoint,)| {
                let access_point = access_point_added(ctx.path(), &access_point);
                ctx.push_msg(access_point);
                println!("added access point");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "RemoveAccessPointEvent",
            ("path",),
            (),
            move |mut ctx, _, access_point: (AccessPoint,)| {
                let access_point = access_point_removed(ctx.path(), &access_point);
                ctx.push_msg(access_point);
                println!("removed access point");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ChangeAccessPointEvent",
            ("path",),
            (),
            move |mut ctx, _, map: (PropMap,)| {
                let map = access_point_changed(ctx.path(), &map);
                ctx.push_msg(map);
                println!("changed access point");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "AddBluetoothDeviceEvent",
            ("device",),
            (),
            move |mut ctx, _, (device,): (BluetoothDevice,)| {
                let device = bluetooth_device_added(ctx.path(), &(device,));
                ctx.push_msg(device);
                println!("added bluetooth device");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "RemoveBluetoothDeviceEvent",
            ("path",),
            (),
            move |mut ctx, _, (path,): (Path<'static>,)| {
                let path = bluetooth_device_removed(ctx.path(), &(path,));
                ctx.push_msg(path);
                println!("removed bluetooth device");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "AddSinkEvent",
            ("sink",),
            (),
            move |mut ctx, _, (sink,): (Sink,)| {
                let sink = sink_added(ctx.path(), &(sink,));
                ctx.push_msg(sink);
                println!("added sink");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "RemoveSinkEvent",
            ("sink",),
            (),
            move |mut ctx, _, (sink,): (Sink,)| {
                let sink = sink_removed(ctx.path(), &(sink,));
                ctx.push_msg(sink);
                println!("removed sink");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ChangedSinkEvent",
            ("sink",),
            (),
            move |mut ctx, _, (sink,): (Sink,)| {
                let sink = sink_changed(ctx.path(), &(sink,));
                ctx.push_msg(sink);
                println!("changed sink");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "AddSourceEvent",
            ("source",),
            (),
            move |mut ctx, _, (source,): (Source,)| {
                let source = source_added(ctx.path(), &(source,));
                ctx.push_msg(source);
                println!("added source");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "RemoveSourceEvent",
            ("source",),
            (),
            move |mut ctx, _, (source,): (Source,)| {
                let source = source_removed(ctx.path(), &(source,));
                ctx.push_msg(source);
                println!("removed source");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ChangedSourceEvent",
            ("source",),
            (),
            move |mut ctx, _, (source,): (Source,)| {
                let source = source_changed(ctx.path(), &(source,));
                ctx.push_msg(source);
                println!("changed source");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "AddInputStreamEvent",
            ("input_stream",),
            (),
            move |mut ctx, _, (input_stream,): (InputStream,)| {
                let input_stream = input_stream_added(ctx.path(), &(input_stream,));
                ctx.push_msg(input_stream);
                println!("added input stream");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "RemoveInputStreamEvent",
            ("input_stream",),
            (),
            move |mut ctx, _, (input_stream,): (InputStream,)| {
                let input_stream = input_stream_removed(ctx.path(), &(input_stream,));
                ctx.push_msg(input_stream);
                println!("removed input stream");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ChangedInputStream",
            ("input_stream",),
            (),
            move |mut ctx, _, (input_stream,): (InputStream,)| {
                let input_stream = input_stream_changed(ctx.path(), &(input_stream,));
                ctx.push_msg(input_stream);
                println!("changed input stream");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "AddOutputStreamEvent",
            ("output_stream",),
            (),
            move |mut ctx, _, (output_stream,): (OutputStream,)| {
                let output_stream = output_stream_added(ctx.path(), &(output_stream,));
                ctx.push_msg(output_stream);
                println!("added output stream");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "RemoveOutputStreamEvent",
            ("output_stream",),
            (),
            move |mut ctx, _, (output_stream,): (OutputStream,)| {
                let output_stream = output_stream_removed(ctx.path(), &(output_stream,));
                ctx.push_msg(output_stream);
                println!("removed output stream");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ChangedOutputStreamEvent",
            ("ouput_stream",),
            (),
            move |mut ctx, _, (output_stream,): (OutputStream,)| {
                let output_stream = output_stream_changed(ctx.path(), &(output_stream,));
                ctx.push_msg(output_stream);
                println!("changed output stream");
                async move { ctx.reply(Ok(())) }
            },
        );
    });
    cross.insert("/org/xetibo/ReSet", &[token], data);

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
