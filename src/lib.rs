pub mod api;
mod audio;
mod bluetooth;
mod network;
mod tests;
mod utils;

use std::{
    future::{self},
    process::exit,
};

use dbus::{channel::MatchingReceiver, message::MatchRule, Path};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection::{self};
use re_set_lib::utils::call_system_dbus_method;
use utils::{AudioRequest, AudioResponse, BASE};

use crate::{
    audio::audio_manager_dbus::setup_audio_manager,
    bluetooth::bluetooth_manager_dbus::setup_bluetooth_manager,
    network::network_manager_dbus::setup_wireless_manager,
    utils::{DaemonData, DBUS_PATH},
};

/// # Running the daemon as a library function
///
/// Used as a standalone binary:
/// ```no_run
/// use reset_daemon::run_daemon;
///
/// #[tokio::main]
/// pub async fn main() {
///     run_daemon().await;
/// }
/// ```
///
/// The daemon will run to infinity, so it might be a good idea to put it into a different thread.
/// ```no_run
/// use reset_daemon::run_daemon;
/// tokio::task::spawn(run_daemon());
/// // your other code here...
/// ```
pub async fn run_daemon() {
    let res = connection::new_session_sync();
    if res.is_err() {
        return;
    }
    let (resource, conn) = res.unwrap();

    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    let data = DaemonData::create(_handle, conn.clone()).await;
    if data.is_err() {
        return;
    }
    let data = data.unwrap();

    conn.request_name(BASE, false, true, false).await.unwrap();
    let mut cross = Crossroads::new();
    cross.set_async_support(Some((
        conn.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    let res = call_system_dbus_method::<(), ()>(
        "org.freedesktop.NetworkManager",
        Path::from("/org/freedesktop/NetworkManager"),
        "Introspect",
        "org.freedesktop.DBus.Introspectable",
        (),
        1,
    );
    let wifi_enabled = res.is_ok();
    let res = call_system_dbus_method::<(), ()>(
        "org.bluez",
        Path::from("/org/bluez"),
        "Introspect",
        "org.freedesktop.DBus.Introspectable",
        (),
        1,
    );
    let bluetooth_enabled = res.is_ok();

    let mut features = Vec::new();
    let mut feature_strings = Vec::new();
    if wifi_enabled {
        features.push(setup_wireless_manager(&mut cross));
        feature_strings.push("WiFi");
    }
    if bluetooth_enabled {
        features.push(setup_bluetooth_manager(&mut cross));
        // the agent is currently not implemented
        // features.push(setup_bluetooth_agent(&mut cross));
        feature_strings.push("Bluetooth");
    }
    features.push(setup_audio_manager(&mut cross));
    feature_strings.push("Audio");
    features.push(setup_base(&mut cross, feature_strings));

    cross.insert(DBUS_PATH, &features, data);

    // register bluetooth agent before start
    // will be uncommented when agent is fully functional
    // {
    //     let data: &mut DaemonData = cross.data_mut(&Path::from(DBUS_PATH)).unwrap();
    //     if data.b_interface.current_adapter != Path::from("/") {
    //         // register bluetooth agent before listening to calls
    //         data.b_interface.register_agent();
    //     }
    // }

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

fn setup_base(
    cross: &mut Crossroads,
    features: Vec<&'static str>,
) -> dbus_crossroads::IfaceToken<DaemonData> {
    cross.register(BASE, |c| {
        c.method("GetCapabilities", (), ("capabilities",), move |_, _, ()| {
            // later, this should be handled dymanically -> plugin check
            Ok((features.clone(),))
        });
        c.method("APIVersion", (), ("api-version",), move |_, _, ()| {
            // let the client handle the mismatch -> e.g. they decide if they want to keep using
            // the current daemon or not.
            Ok(("0.3.9",))
        });
        c.method(
            "RegisterClient",
            ("client_name",),
            ("result",),
            move |_, data: &mut DaemonData, (client_name,): (String,)| {
                data.clients.insert(client_name, data.clients.len());
                Ok((true,))
            },
        );
        c.method(
            "UnregisterClient",
            ("client_name",),
            ("result",),
            move |_, data: &mut DaemonData, (client_name,): (String,)| {
                data.clients.remove(&client_name);
                Ok((true,))
            },
        );
        c.method("Shutdown", (), (), move |_, data: &mut DaemonData, ()| {
            data.b_interface.unregister_agent();
            data.handle.abort();
            exit(0);
            #[allow(unreachable_code)]
            Ok(())
        });
    })
}
