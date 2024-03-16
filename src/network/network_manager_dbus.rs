use std::{collections::HashMap, sync::atomic::Ordering, thread, time::Duration};

use dbus::{arg::PropMap, blocking::Connection, Path};
use dbus_crossroads::Crossroads;
use re_set_lib::network::network_structures::{AccessPoint, WifiDevice};

use crate::{utils::get_wifi_status, DaemonData};

use super::network_manager::{
    get_connection_settings, get_stored_connections, get_wifi_devices, set_connection_settings,
    set_wifi_enabled, start_listener, stop_listener,
};

pub fn setup_wireless_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register(NETWORK_INTERFACE!(), |c| {
        c.signal::<(AccessPoint,), _>("AccessPointChanged", ("access_point",));
        c.signal::<(AccessPoint,), _>("AccessPointAdded", ("access_point",));
        c.signal::<(Path<'static>,), _>("AccessPointRemoved", ("path",));
        c.signal::<(WifiDevice,), _>("WifiDeviceChanged", ("device",));
        c.method_with_cr_async(
            "ListAccessPoints",
            (),
            ("access_points",),
            move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let device = data.current_n_device.clone();
                async move {
                    let access_points = device.read().unwrap().get_access_points();
                    ctx.reply(Ok((access_points,)))
                }
            }
        );
        c.method_with_cr_async("GetWifiStatus", (), ("status",), move |mut ctx, _, ()| async move {
            ctx.reply(Ok((get_wifi_status(),)))
        });
        // needs blocking
        c.method(
            "SetWifiEnabled",
            ("enabled",),
            ("result",),
            move |_, data: &mut DaemonData, (enabled,): (bool,)| {
                let active_listener = data.network_listener_active.clone();
                let stop_requested = data.network_stop_requested.clone();
                if enabled {
                    if !active_listener.load(Ordering::SeqCst) {
                        let path = data.current_n_device.read().unwrap().dbus_path.clone();
                        let device = data.current_n_device.clone();
                        let connection = data.connection.clone();
                        thread::spawn(move || {
                            start_listener(
                                connection,
                                device,
                                path,
                                active_listener,
                                stop_requested,
                            )
                        });
                    }
                } else {
                    stop_listener(stop_requested);
                }
                Ok((set_wifi_enabled(enabled, data),))
            },
        );
        c.method_with_cr_async(
            "GetCurrentWifiDevice",
            (),
            ("device",),
            move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let device = data.current_n_device.clone();
                async move {
                let device = device.read().unwrap();
                let path = device.dbus_path.clone();
                let name = device.name.clone();
                let active_access_point;
                let active_access_point_opt =
                    device.access_point.clone();
                if let Some(active_access_point_opt) = active_access_point_opt {
                    active_access_point = active_access_point_opt.ssid;
                } else {
                    active_access_point = Vec::new();
                }
                ctx.reply(Ok((WifiDevice {
                    path,
                    name,
                    active_access_point,
                },)))
                }
            },
        );
        c.method_with_cr_async(
            "GetAllWifiDevices",
            (),
            ("devices",),
            move |mut ctx, _, ()| {
                async move {
                let mut devices = Vec::new();
                let device_paths = get_wifi_devices();
                for device in device_paths {
                        let device = device.read().unwrap();
                        let path = device.dbus_path.clone();
                        let name = device.name.clone();
                    let active_access_point;
                    let active_access_point_opt =
                        device.access_point.clone();
                    if let Some(active_access_point_opt) = active_access_point_opt {
                        active_access_point = active_access_point_opt.ssid;
                    } else {
                        active_access_point = Vec::new();
                    }
                    devices.push(WifiDevice {
                        path,
                        name,
                        active_access_point,
                    });
                }
                ctx.reply(Ok((devices,)))
                }
            },
        );
        // needs blocking
        c.method(
            "SetWifiDevice",
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
        c.method_with_cr_async(
            "ConnectToKnownAccessPoint",
            ("access_point",),
            ("result",),
            move |mut ctx, cross, (access_point,): (AccessPoint,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let device = data.current_n_device.clone();
                async move {
                    let res = device
                        .write()
                        .unwrap()
                        .connect_to_access_point(access_point);
                    ctx.reply(Ok((res.is_ok(),)))
                }
            },
        );
        c.method_with_cr_async(
            "ConnectToNewAccessPoint",
            ("access_point", "password"),
            ("result",),
            move |mut ctx, cross, (access_point, password): (AccessPoint, String)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let device = data.current_n_device.clone();
                async move {
                    let res = device
                        .write()
                        .unwrap()
                        .add_and_connect_to_access_point(access_point, password);
                    ctx.reply(Ok((res.is_ok(),)))
                }
            }
        );
        c.method_with_cr_async(
            "DisconnectFromCurrentAccessPoint",
            (),
            ("result",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let device = data.current_n_device.clone();
                async move {
                let res = device
                    .write()
                    .unwrap()
                    .disconnect_from_current();
                    ctx.reply(Ok((res.is_ok(),)))
                }
            },
        );
        c.method_with_cr_async("ListStoredConnections", (), ("result",), move |mut ctx, _, ()| async move {
            let res = get_stored_connections();
            ctx.reply(Ok((res,)))
        });
        c.method_with_cr_async(
            "GetConnectionSettings",
            ("path",),
            ("result",),
            move |mut ctx, _, (path,): (Path<'static>,)| async move {
                let res = get_connection_settings(path);
                if res.is_err() {
                    return ctx.reply(Err(dbus::MethodErr::invalid_arg(
                        "Could not get settings for this connection.",
                    ),));
                }
                ctx.reply(Ok((res.unwrap(),)))
            },
        );
        c.method_with_cr_async(
            "SetConnectionSettings",
            ("path", "settings"),
            ("result",),
            move |mut ctx, _, (path, settings): (Path<'static>, HashMap<String, PropMap>)| async move {
                ctx.reply(Ok((set_connection_settings(path, settings),)))
            },
        );
        c.method_with_cr_async(
            "DeleteConnection",
            ("path",),
            ("result",),
            move |mut ctx, _, (path,): (Path<'static>,)| async move {
                let res = dbus_method!(
                    NM_INTERFACE_BASE!(),
                    path,
                    "Delete",
                    NM_SETTINGS_INTERFACE!(),
                    (),
                    1000,
                    (),
            );
                let result = res.is_ok();
                ctx.reply(Ok((result,)))
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
                let stop_requested = data.network_stop_requested.clone();
                let device = data.current_n_device.clone();
                let connection = data.connection.clone();
                let mut result = true;
                {
                    if device.read().unwrap().dbus_path.is_empty()
                        || active_listener.load(Ordering::SeqCst)
                    {
                        result = false;
                    } else {
                        thread::spawn(move || {
                            let res = start_listener(
                                connection,
                                device,
                                path,
                                active_listener,
                                stop_requested,
                            );
                            if res.is_err() {
                                println!("{}", res.err().unwrap());
                            }
                        });
                    }
                }
                async move { ctx.reply(Ok((result,))) }
            },
        );
        c.method_with_cr_async(
            "StopNetworkListener",
            (),
            ("result",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let stop_requested = data.network_stop_requested.clone();
                async move {
                    stop_listener(stop_requested);
                    ctx.reply(Ok((true,)))
                }
            },
        );
    });
    token
}
