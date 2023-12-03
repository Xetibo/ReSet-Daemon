use std::{rc::Rc, sync::atomic::Ordering};

use dbus::Path;
use dbus_crossroads::Crossroads;
use ReSet_Lib::bluetooth::bluetooth::{BluetoothAdapter, BluetoothDevice};

use crate::DaemonData;

use super::bluetooth_manager::get_connections;

pub fn setup_bluetooth_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register("org.Xetibo.ReSetBluetooth", |c| {
        c.signal::<(BluetoothDevice,), _>("BluetoothDeviceAdded", ("device",));
        c.signal::<(Path<'static>,), _>("BluetoothDeviceRemoved", ("path",));
        c.signal::<(BluetoothDevice,), _>("BluetoothDeviceChanged", ("device",));
        c.signal::<(), _>("PincodeRequested", ());
        c.signal::<(String,), _>("DisplayPinCode", ("code",));
        c.signal::<(), _>("PassKeyRequested", ());
        c.signal::<(u32, u16), _>("DisplayPassKey", ("passkey", "entered"));
        c.signal::<(), _>("PinCodeRequested", ());
        c.method_with_cr_async(
            "StartBluetoothScan",
            ("duration",),
            (),
            move |mut ctx, cross, (duration,): (u32,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                data.b_interface.start_bluetooth_discovery();
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "StopBluetoothScan",
            ("duration",),
            (),
            move |mut ctx, cross, (duration,): (u32,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let active_listener = data.network_listener_active.clone();
                data.b_interface
                    .start_bluetooth_listener(duration as u64, active_listener);
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "StartBluetoothListener",
            ("duration",),
            (),
            move |mut ctx, cross, (duration,): (u32,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let active_listener = data.network_listener_active.clone();
                data.b_interface
                    .start_bluetooth_listener(duration as u64, active_listener);
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method(
            "StopBluetoothListener",
            (),
            ("result",),
            move |_, d: &mut DaemonData, ()| {
                let active_listener = d.network_listener_active.clone();
                if !active_listener.load(Ordering::SeqCst) {
                    return Ok((false,));
                }
                let res = d.b_interface.stop_bluetooth_discovery();
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((res.is_ok(),))
            },
        );
        c.method(
            "GetBluetoothAdapters",
            (),
            ("adapters",),
            move |_, d: &mut DaemonData, ()| Ok((d.b_interface.adapters.clone(),)),
        );
        c.method(
            "GetCurrentBluetoothAdapter",
            (),
            ("adapter",),
            move |_, d: &mut DaemonData, ()| Ok((d.b_interface.current_adapter.clone(),)),
        );
        c.method(
            "SetBluetoothAdapter",
            ("path",),
            ("result",),
            move |_, d: &mut DaemonData, (path,): (Path<'static>,)| {
                for adapter in d.b_interface.adapters.iter() {
                    if adapter.path == path {
                        d.b_interface.current_adapter = adapter.clone();
                        return Ok((true,));
                    }
                }
                Ok((false,))
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
                    dbg!(res);
                    return Ok((false,));
                }
                dbg!(res);
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
        c.method(
            "RemoveDevicePairing",
            ("path",),
            ("result",),
            move |_, d: &mut DaemonData, (path,): (Path<'static>,)| {
                let res = d.b_interface.remove_device_pairing(path);
                if res.is_err() {
                    return Ok((false,));
                }
                Ok((true,))
            },
        );
        c.method(
            "GetConnectedBluetoothDevices",
            (),
            ("devices",),
            move |_, _, ()| Ok((get_connections(),)),
        );
    });
    token
}
