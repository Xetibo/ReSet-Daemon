use std::{
    future,
    rc::Rc,
    sync::{Arc, Mutex},
};

use dbus::{
    blocking::Connection, channel::MatchingReceiver, message::MatchRule, nonblock::SyncConnection,
    Path,
};
use dbus_crossroads::{Context, Crossroads};
use dbus_tokio::connection::{self, IOResource};
use tokio;

use super::{
    bluetooth::{BluetoothDevice, BluetoothInterface},
    network::{get_wifi_devices, AccessPoint, Device, Error},
};

#[derive(Clone)]
pub struct DaemonData {
    pub n_devices: Vec<Device>,
    pub current_n_device: Device,
    pub b_interface: BluetoothInterface,
}

pub struct Daemon {
    pub data: DaemonData,
}

impl Daemon {
    pub fn create() -> Result<Self, Error> {
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
        Ok(Self {
            data: DaemonData {
                n_devices,
                current_n_device,
                b_interface,
            },
        })
    }

    pub async fn run(&mut self) {
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
        });
        cross.insert("/org/xetibo/ReSet/Network", &[token], self.data.clone());
        cross.insert("/org/xetibo/ReSet/Bluetooth", &[token], self.data.clone());

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
}
