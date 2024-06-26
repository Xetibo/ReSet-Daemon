use std::{collections::HashMap, future, sync::atomic::AtomicBool};

use dbus::{channel::MatchingReceiver, message::MatchRule};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;
use re_set_lib::utils::variant::Variant;

use crate::mock::{bluetooth::MockBluetooth, network::mock_network_manager};

use crate::mock::{bluetooth::MockBluetoothData, network::MockNetworkManager};

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

    // let mut mock_implementations = mock_network_interface(&mut cross);
    let mut mock_implementations = Vec::new();
    let mock_network_manager = mock_network_manager(&mut cross, conn.clone());
    let mock_bluetooth = MockBluetooth::new(&mut cross);
    mock_implementations.push(mock_network_manager.network_manager_base);
    mock_implementations.push(mock_network_manager.network_manager_settings);
    // mock_implementations.push(mock_network_manager.network_manager_active_connection);
    // mock_implementations.push(mock_network_manager.network_manager_base);
    // mock_implementations.push(mock_network_manager.network_manager_base);
    // mock_sound_interface(&mut cross),
    // load all plugin implementations

    // cross.object_manager();
    cross.insert(
        DBUS_PATH_TEST!(),
        &mock_implementations,
        MockTestData {
            network_data: mock_network_manager,
            bluetooth_data: mock_bluetooth.data,
            plugin_data: HashMap::new(),
        },
    );
    // needed for bluetooth
    cross.insert("/", [&cross.object_manager()], ());

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
    pub network_data: MockNetworkManager,
    pub bluetooth_data: MockBluetoothData,
    pub plugin_data: HashMap<String, Variant>,
}

unsafe impl Send for MockTestData {}
unsafe impl Sync for MockTestData {}
