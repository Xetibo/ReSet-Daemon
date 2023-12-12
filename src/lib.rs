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
use utils::{AudioRequest, AudioResponse, BASE};

use crate::{
    audio::audio_manager_dbus::setup_audio_manager,
    bluetooth::{
        bluetooth_agent_dbus::setup_bluetooth_agent,
        bluetooth_manager_dbus::setup_bluetooth_manager,
    },
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

    conn.request_name(BASE, false, true, false)
        .await
        .unwrap();
    let mut cross = Crossroads::new();
    cross.set_async_support(Some((
        conn.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    let base = setup_base(&mut cross);
    let wireless_manager = setup_wireless_manager(&mut cross);
    let bluetooth_manager = setup_bluetooth_manager(&mut cross);
    let bluetooth_agent = setup_bluetooth_agent(&mut cross);
    let audio_manager = setup_audio_manager(&mut cross);


    cross.insert(
        DBUS_PATH,
        &[
            base,
            wireless_manager,
            bluetooth_manager,
            bluetooth_agent,
            audio_manager,
        ],
        data,
    );

    let data: &mut DaemonData = cross.data_mut(&Path::from(DBUS_PATH)).unwrap();
    // register bluetooth agent before listening to calls
    data.b_interface.register_agent();

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
) -> dbus_crossroads::IfaceToken<DaemonData> {
    cross.register(BASE, |c| {
        c.method("GetCapabilities", (), ("capabilities",), move |_, _, ()| {
            // later, this should be handled dymanically -> plugin check
            Ok((vec!["Bluetooth", "Wifi", "Audio"],))
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
