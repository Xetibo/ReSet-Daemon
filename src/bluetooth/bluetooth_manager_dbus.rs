use std::sync::atomic::Ordering;

use dbus::Path;
use dbus_crossroads::Crossroads;
use re_set_lib::bluetooth::bluetooth_structures::BluetoothDevice;
use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::DaemonData;

use super::bluetooth_manager::{
    get_all_bluetooth_adapters, get_all_bluetooth_devices, get_bluetooth_adapter, get_connections,
    set_adapter_discoverable, set_adapter_enabled, set_adapter_pairable,
};

pub fn setup_bluetooth_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register(BLUETOOTH_INTERFACE!(), |c| {
        c.signal::<(BluetoothDevice,), _>("BluetoothDeviceAdded", ("device",));
        c.signal::<(Path<'static>,), _>("BluetoothDeviceRemoved", ("path",));
        c.signal::<(BluetoothDevice,), _>("BluetoothDeviceChanged", ("device",));
        c.signal::<(), _>("PincodeRequested", ());
        c.signal::<(String,), _>("DisplayPinCode", ("code",));
        c.signal::<(), _>("PassKeyRequested", ());
        c.signal::<(u32, u16), _>("DisplayPassKey", ("passkey", "entered"));
        c.signal::<(), _>("PinCodeRequested", ());
        c.method_with_cr_async("StartBluetoothScan", (), (), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            data.bluetooth_scan_request.store(1, Ordering::SeqCst);
            if !data.bluetooth_listener_active.load(Ordering::SeqCst) {
                data.b_interface
                    .start_bluetooth_discovery(data.bluetooth_scan_active.clone());
            }
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async("StopBluetoothScan", (), (), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            data.bluetooth_scan_request.store(2, Ordering::SeqCst);
            if !data.bluetooth_listener_active.load(Ordering::SeqCst) {
                data.b_interface
                    .stop_bluetooth_discovery(data.bluetooth_scan_active.clone());
            }
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async(
            "StartBluetoothListener",
            (),
            (),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                data.b_interface.start_bluetooth_listener(
                    data.bluetooth_listener_active.clone(),
                    data.bluetooth_scan_request.clone(),
                    data.bluetooth_scan_active.clone(),
                    data.bluetooth_stop_requested.clone(),
                );
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method(
            "StopBluetoothListener",
            (),
            (),
            move |_, d: &mut DaemonData, ()| {
                d.bluetooth_stop_requested.store(true, Ordering::SeqCst);
                Ok(())
            },
        );
        // TODO: test if new version can be used instead
        // c.method(
        //     "GetBluetoothAdapters",
        //     (),
        //     ("adapters",),
        //     move |_, d: &mut DaemonData, ()| {
        //         let mut adapters = Vec::new();
        //         for path in d.b_interface.adapters.iter() {
        //             adapters.push(get_bluetooth_adapter(path));
        //         }
        //         Ok((adapters,))
        //     },
        // );
        c.method(
            "GetBluetoothAdapters",
            (),
            ("adapters",),
            move |_, _, ()| Ok((get_all_bluetooth_adapters(),)),
        );
        c.method(
            "GetCurrentBluetoothAdapter",
            (),
            ("adapter",),
            move |_, d: &mut DaemonData, ()| {
                Ok((get_bluetooth_adapter(&d.b_interface.current_adapter),))
            },
        );
        c.method(
            "SetBluetoothAdapter",
            ("path",),
            ("result",),
            move |_, d: &mut DaemonData, (path,): (Path<'static>,)| {
                for adapter in d.b_interface.adapters.iter() {
                    if *adapter == path {
                        d.b_interface.current_adapter = adapter.clone();
                        return Ok((true,));
                    }
                }
                Ok((false,))
            },
        );
        c.method(
            "SetBluetoothAdapterEnabled",
            ("path", "enabled"),
            ("result",),
            move |_, _, (path, enabled): (Path<'static>, bool)| {
                Ok((set_adapter_enabled(path, enabled),))
            },
        );
        c.method(
            "SetBluetoothAdapterDiscoverability",
            ("path", "enabled"),
            ("result",),
            move |_, _, (path, enabled): (Path<'static>, bool)| {
                Ok((set_adapter_discoverable(path, enabled),))
            },
        );
        c.method(
            "SetBluetoothAdapterPairability",
            ("path", "enabled"),
            ("result",),
            move |_, _, (path, enabled): (Path<'static>, bool)| {
                Ok((set_adapter_pairable(path, enabled),))
            },
        );
        c.method("GetBluetoothDevices", (), ("devices",), move |_, _, ()| {
            Ok((get_all_bluetooth_devices(),))
        });
        c.method(
            "ConnectToBluetoothDevice",
            ("device",),
            ("result",),
            move |_, d: &mut DaemonData, (device,): (Path<'static>,)| {
                d.b_interface.connect_to(device);
                Ok((true,))
            },
        );
        // TODO pairing does not work this way
        // figure out how pairing works
        // c.method(
        //     "PairWithBluetoothDevice",
        //     ("device",),
        //     ("result",),
        //     move |_, d: &mut DaemonData, (device,): (Path<'static>,)| {
        //         println!("pair called");
        //         let res = d.b_interface.pair_with(device);
        //         // if res.is_err() {
        //         // println!("pair called");
        //         //     return Ok((false,));
        //         // }
        //         Ok((true,))
        //     },
        // );
        c.method(
            "DisconnectFromBluetoothDevice",
            ("device",),
            ("result",),
            move |_, d: &mut DaemonData, (device,): (Path<'static>,)| {
                let res = d.b_interface.disconnect(device.clone());
                if res.is_err() {
                    ERROR!(
                        format!("Could not disconnect from device: {}", device),
                        ErrorLevel::PartialBreakage
                    );
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
                let res = d.b_interface.remove_device_pairing(path.clone());
                if res.is_err() {
                    ERROR!(
                        format!("Could not remove device pairing: {}", path),
                        ErrorLevel::PartialBreakage
                    );
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
