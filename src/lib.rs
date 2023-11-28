pub mod api;
mod audio;
mod bluetooth;
mod network;
mod tests;
mod utils;

use std::future::{self};

use dbus::{channel::MatchingReceiver, message::MatchRule};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection::{self};
use utils::{AudioRequest, AudioResponse};

use crate::{
    audio::audio_manager_dbus::setup_audio_manager,
    bluetooth::{
        bluetooth_agent_dbus::setup_bluetooth_agent,
        bluetooth_manager_dbus::setup_bluetooth_manager,
    },
    network::network_manager_dbus::setup_wireless_manager,
    utils::DaemonData,
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
    let data = DaemonData::create(conn.clone()).await;
    if data.is_err() {
        return;
    }
    let data = data.unwrap();

    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    conn.request_name("org.Xetibo.ReSetDaemon", false, true, false)
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
        "/org/Xetibo/ReSetDaemon",
        &[
            base,
            wireless_manager,
            bluetooth_manager,
            bluetooth_agent,
            audio_manager,
        ],
        data,
    );

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

fn setup_base(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    cross.register("org.Xetibo.ReSetDaemon", |c| {
        c.method("Check", (), ("result",), move |_, _, ()| Ok((true,)));
    })
}
