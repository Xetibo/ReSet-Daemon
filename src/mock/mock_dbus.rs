use std::{collections::HashMap, future, sync::atomic::AtomicBool};

use dbus::{channel::MatchingReceiver, message::MatchRule};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;

use crate::mock::{bluetooth::mock_bluetooth_interface, network::mock_network_interface};

use super::{bluetooth::MockBluetoothData, network::MockNetworkData, variant::MockVariant};

const MOCK_BASE: &str = "org.Xetibo.ReSet.Test";
const MOCK_DBUS_PATH: &str = "/org/Xetibo/ReSet/Test";

pub async fn start_mock_implementation_server(ready: AtomicBool) {
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

    let mut mock_implementations = mock_network_interface(&mut cross);
    mock_implementations.push(mock_bluetooth_interface(&mut cross));
    // mock_sound_interface(&mut cross),
    // load all plugin implementations

    cross.insert(
        MOCK_DBUS_PATH,
        &mock_implementations,
        MockTestData {
            network_data: MockNetworkData::new(),
            bluetooth_data: MockBluetoothData::new(),
            plugin_data: HashMap::new(),
        },
    );

    conn.start_receive(
        MatchRule::new_method_call(),
        Box::new(move |msg, conn| {
            cross.handle_message(msg, conn).unwrap();
            true
        }),
    );

    ready.store(true, std::sync::atomic::Ordering::SeqCst);

    future::pending::<()>().await;
    unreachable!()
}

pub struct MockTestData {
    network_data: MockNetworkData,
    bluetooth_data: MockBluetoothData,
    plugin_data: HashMap<String, MockVariant>,
}

unsafe impl Send for MockTestData {}
unsafe impl Sync for MockTestData {}
