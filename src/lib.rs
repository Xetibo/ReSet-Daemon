#![feature(trait_upcasting)]
#[macro_use]
mod macros;
pub mod api;
mod audio;
mod bluetooth;
pub mod mock;
mod network;
mod tests;
pub mod utils;

use std::{fs, future, process::exit, time::Duration};

use dbus::blocking::Connection;
use dbus::{channel::MatchingReceiver, message::MatchRule, Path};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;
use re_set_lib::{write_log_to_file, LOG};
use utils::{AudioRequest, AudioResponse, BASE};

use crate::{
    audio::audio_manager_dbus::setup_audio_manager,
    bluetooth::bluetooth_manager_dbus::setup_bluetooth_manager,
    network::network_manager_dbus::setup_wireless_manager, utils::DaemonData,
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
    create_log_file();
    LOG!("/tmp/reset_daemon_log", "Running in debug mode\n");
    let res = connection::new_session_sync();
    if res.is_err() {
        return;
    }
    let (resource, conn) = res.unwrap();

    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    conn.request_name(BASE, false, true, false).await.unwrap();
    let mut cross = Crossroads::new();
    cross.set_async_support(Some((
        conn.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        Path::from(NM_PATH!()),
        "Introspect",
        "org.freedesktop.DBus.Introspectable",
        (),
        1,
        (),
    );
    let wifi_enabled = res.is_ok();
    let res = dbus_method!(
        BLUEZ_INTERFACE!(),
        "/",
        "Introspect",
        "org.freedesktop.DBus.Introspectable",
        (),
        1,
        (),
    );
    let bluetooth_enabled = res.is_ok();

    let mut features = Vec::new();
    let mut feature_strings = Vec::new();
    if wifi_enabled {
        features.push(setup_wireless_manager(&mut cross));
        feature_strings.push("WiFi");
        LOG!("/tmp/reset_daemon_log", "WiFi feature started\n");
    }
    if bluetooth_enabled {
        features.push(setup_bluetooth_manager(&mut cross));
        // the agent is currently not implemented
        // features.push(setup_bluetooth_agent(&mut cross));
        feature_strings.push("Bluetooth");
        LOG!("/tmp/reset_daemon_log", "Bluetooth feature started\n");
    }
    // TODO: how to check for audio?
    features.push(setup_audio_manager(&mut cross));
    feature_strings.push("Audio");

    let data = DaemonData::create(_handle, conn.clone(), &feature_strings);
    if data.is_err() {
        return;
    }
    let data = data.unwrap();

    features.push(setup_base(&mut cross, feature_strings));

    cross.insert(DBUS_PATH!(), &features, data);

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

fn create_log_file() {
    fs::File::create("/tmp/reset_daemon_log").expect("Could not create log file.");
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
            Ok(("1.0.1",))
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
            let _ = data.audio_sender.send(AudioRequest::StopListener);
            exit(0);
            #[allow(unreachable_code)]
            Ok(())
        });
    })
}
