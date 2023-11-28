use dbus::Path;
use dbus_crossroads::Crossroads;
use ReSet_Lib::bluetooth::bluetooth::BluetoothDevice;

use crate::DaemonData;

use super::bluetooth_manager::get_connections;

pub fn setup_bluetooth_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register("org.Xetibo.ReSetBluetooth", |c| {
        c.signal::<(BluetoothDevice,), _>("BluetoothDeviceAdded", ("device",));
        c.signal::<(Path<'static>,), _>("BluetoothDeviceRemoved", ("path",));
        c.method_with_cr_async(
            "StartBluetoothListener",
            ("duration",),
            (),
            move |mut ctx, cross, (duration,): (u32,)| {
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
            "StopBluetoothListener",
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
        c.method(
            "GetConnectedBluetoothDevices",
            (),
            ("devices",),
            move |_, _, ()| Ok((get_connections(),)),
        );
    });
    token
}
