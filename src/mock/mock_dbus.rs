use std::future;

use dbus::{channel::MatchingReceiver, message::MatchRule};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;

use crate::mock::network::mock_network_interface;

const MOCK_BASE: &'static str = "MOCKbase";
const MOCK_DBUS_PATH: &'static str = "MOCKDbusPath";

pub async fn start_mock_implementation_server() {
    let res = connection::new_session_sync();
    if res.is_err() {
        return;
    }
    let (_, conn) = res.unwrap();
    conn.request_name(MOCK_BASE, false, true, false)
        .await
        .unwrap();
    let mut cross = Crossroads::new();
    cross.set_async_support(Some((
        conn.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    let mock_implementations = vec![mock_network_interface(&mut cross)];
    // load all plugin implementations

    cross.insert(MOCK_DBUS_PATH, &mock_implementations, MockNetworkData {});

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

pub struct MockNetworkData {}
