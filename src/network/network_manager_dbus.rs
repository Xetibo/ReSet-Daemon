use std::{collections::HashMap, thread};

use dbus::{arg::PropMap, Path};
use dbus_crossroads::Crossroads;
use ReSet_Lib::{
    network::network::AccessPoint,
    utils::{call_system_dbus_method, get_system_dbus_property},
};

use crate::DaemonData;

use super::network_manager::{
    get_connection_settings, get_stored_connections, get_wifi_devices, list_connections,
    set_connection_settings, start_listener, stop_listener,
};

pub fn setup_wireless_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
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
                Ok((true,))
            },
        );
    });
    token
}
