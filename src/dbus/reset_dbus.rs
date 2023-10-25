use std::{
    future::{self},
    sync::{Arc, Mutex},
    thread,
};

use dbus::{channel::MatchingReceiver, message::MatchRule, Path};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection::{self};
use tokio;

use super::{
    audio::{Sink, Source},
    bluetooth::{BluetoothDevice, BluetoothInterface},
};
use std::sync::mpsc::{self, Receiver, Sender};

use super::{
    audio::PulseServer,
    network::{get_wifi_devices, AccessPoint, Device, Error},
};

pub enum Request {
    ListSources,
    ListSinks,
}

pub enum Response {
    Sources(Vec<Source>),
    Sinks(Vec<Sink>),
}

pub struct DaemonData {
    pub n_devices: Vec<Device>,
    pub current_n_device: Device,
    pub b_interface: BluetoothInterface,
    pub sender: Sender<Request>,
    pub receiver: Receiver<Response>,
}
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

        let (dbus_sender, pulse_receiver): (Sender<Request>, Receiver<Request>) = mpsc::channel();
        let (pulse_sender, dbus_receiver): (Sender<Response>, Receiver<Response>) = mpsc::channel();

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
            sender: dbus_sender,
            receiver: dbus_receiver,
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
        let _bluetooth_device_added = c
            .signal::<(Path<'static>, BluetoothDevice), _>(
                "BluetoothDeviceAdded",
                ("path", "device"),
            )
            .msg_fn();
        c.method(
            "ListAccessPoints",
            (),
            ("access_points",),
            move |_, d: &mut DaemonData, ()| {
                let access_points = d.current_n_device.get_access_points();
                dbg!(access_points.clone());
                Ok((d.current_n_device.get_access_points(),))
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
        c.method_with_cr_async(
            "StartBluetoothSearch",
            (),
            ("result",),
            move |ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let ctx_ref = Arc::new(Mutex::new(ctx));
                let res = data.b_interface.start_discovery(ctx_ref.clone());
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
        c.method_with_cr_async("ListSinks", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let sinks: Vec<Sink>;
            let _ = data.sender.send(Request::ListSinks);
            let response = data.receiver.recv();
            if response.is_ok() {
                sinks = match response.unwrap() {
                    Response::Sinks(s) => s,
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
            let _ = data.sender.send(Request::ListSources);
            let response = data.receiver.recv();
            if response.is_ok() {
                sources = match response.unwrap() {
                    Response::Sources(s) => s,
                    _ => Vec::new(),
                }
            } else {
                sources = Vec::new();
            }
            async move { ctx.reply(Ok((sources,))) }
        });
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
