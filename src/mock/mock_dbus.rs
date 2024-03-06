use std::{collections::HashMap, future, sync::atomic::AtomicBool};

use dbus::{channel::MatchingReceiver, message::MatchRule};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;

use crate::mock::{bluetooth::mock_bluetooth_interface, network::mock_network_interface};

use super::{bluetooth::MockBluetoothData, network::MockNetworkData, variant::MockVariant};

pub async fn start_mock_implementation_server(ready: &AtomicBool) {
    let res = connection::new_session_sync();
    if res.is_err() {
        return;
    }
    let (resource, conn) = res.unwrap();

    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    conn.request_name(BASE_TEST_INTERFACE!(), false, true, false)
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
        DBUS_PATH_TEST!(),
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
    pub network_data: MockNetworkData,
    pub bluetooth_data: MockBluetoothData,
    pub plugin_data: HashMap<String, MockVariant>,
}

unsafe impl Send for MockTestData {}
unsafe impl Sync for MockTestData {}
