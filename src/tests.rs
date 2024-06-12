use crate::{mock::mock_dbus::start_mock_implementation_server, BACKEND_PLUGINS};
use crate::{run_daemon, utils::AUDIO};
use dbus::{
    arg::{AppendAll, ReadAll},
    blocking::Connection,
    Path,
};

use once_cell::sync::Lazy;
use serial_test::serial;

use re_set_lib::audio::audio_structures::Sink;
use re_set_lib::audio::audio_structures::{InputStream, OutputStream, Source};
use re_set_lib::bluetooth::bluetooth_structures::BluetoothDevice;
use re_set_lib::network::network_structures::AccessPoint;

use std::sync::Arc;
use std::{
    hint,
    sync::atomic::{AtomicBool, AtomicU16, Ordering},
};
use std::{thread, time::Duration};
use tokio::runtime;

static COUNTER: AtomicU16 = AtomicU16::new(0);
static READY: AtomicBool = AtomicBool::new(false);
static DAEMON_READY: Lazy<Option<Arc<AtomicBool>>> =
    Lazy::new(|| Some(Arc::new(AtomicBool::new(false))));

fn call_session_dbus_method<
    I: AppendAll + Sync + Send + 'static,
    O: ReadAll + Sync + Send + 'static,
>(
    function: &str,
    proxy_name: &str,
    params: I,
) -> Result<O, dbus::Error> {
    let conn = Connection::new_session();
    let conn = conn.unwrap();
    let proxy = conn.with_proxy(BASE_INTERFACE!(), DBUS_PATH!(), Duration::from_millis(2000));
    let result: Result<O, dbus::Error> = proxy.method_call(proxy_name, function, params);
    result
}

#[cfg(test)]
fn setup() {
    if COUNTER.fetch_add(1, Ordering::SeqCst) < 1 {
        thread::spawn(|| {
            let rt2 = runtime::Runtime::new().expect("Failed to create runtime");
            rt2.spawn(start_mock_implementation_server(&READY));
            while !READY.load(Ordering::SeqCst) {
                hint::spin_loop();
            }
            let rt = runtime::Runtime::new().expect("Failed to create runtime");
            rt.spawn(run_daemon(DAEMON_READY.clone()));
            while COUNTER.load(Ordering::SeqCst) != 0 {
                hint::spin_loop();
            }
            rt.shutdown_background();
        });
    };
    while !READY.load(Ordering::SeqCst) {
        hint::spin_loop();
    }
    while !DAEMON_READY.clone().unwrap().load(Ordering::SeqCst) {
        hint::spin_loop();
    }
}

#[tokio::test]
// tests fetching bluetooth devices
async fn test_bluetooth_get_devices() {
    setup();
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "StartBluetoothScan",
        BLUETOOTH_INTERFACE!(),
        (),
        1000,
        (),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "GetBluetoothDevices",
        BLUETOOTH_INTERFACE!(),
        (),
        1000,
        (Vec<BluetoothDevice>,),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    assert!(!res.unwrap().0.is_empty());
}

#[tokio::test]
// tests the existance of the mock implementation
async fn test_mock_connection() {
    setup();
    let conn = Connection::new_session();
    let conn = conn.unwrap();
    let proxy = conn.with_proxy(
        BASE_TEST_INTERFACE!(),
        DBUS_PATH_TEST!(),
        Duration::from_millis(2000),
    );
    let res: Result<(), dbus::Error> = proxy.method_call(NM_INTERFACE!(), "Test", ());
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
}

#[tokio::test]
// tests receiving a list of connections through both the mock implementation and the ReSet Daemon
async fn test_list_connections() {
    setup();
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ListAccessPoints",
        NM_INTERFACE_TEST!(),
        (),
        1000,
        (Vec<AccessPoint>,),
    );
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    assert!(!res.unwrap().0.is_empty());
}

#[tokio::test]
#[serial]
// tests adding and removing an access point
async fn test_add_access_point_event() {
    setup();
    dbus_method!(
        BASE_TEST_INTERFACE!(),
        NM_DEVICES_PATH!().to_string() + "/2",
        "CreateFakeAddedSignal",
        NM_DEVICE_INTERFACE!(),
        (),
        1000,
        (),
    )
    .expect("Could not add access point");
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ListAccessPoints",
        NM_INTERFACE_TEST!(),
        (),
        1000,
        (Vec<AccessPoint>,),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    assert_eq!(res.unwrap().0.len(), 2);
    dbus_method!(
        BASE_TEST_INTERFACE!(),
        NM_DEVICES_PATH!().to_string() + "/2",
        "CreateFakeRemovedSignal",
        NM_DEVICE_INTERFACE!(),
        (),
        1000,
        (),
    )
    .expect("Could not remove access point");
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ListAccessPoints",
        NM_INTERFACE_TEST!(),
        (),
        1000,
        (Vec<AccessPoint>,),
    );
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    assert_eq!(res.unwrap().0.len(), 1);
}

#[tokio::test]
#[serial]
// tests connecting to a new access point with a password
async fn test_connect_to_new_access_point() {
    setup();
    connect_to_new_access_point();
    connect_to_known_access_point();
}

fn connect_to_new_access_point() {
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ListAccessPoints",
        NM_INTERFACE_TEST!(),
        (),
        1000,
        (Vec<AccessPoint>,),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    let access_point = res
        .expect("Failed to get access points")
        .0
        .first()
        .unwrap()
        .clone();
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ConnectToNewAccessPoint",
        NM_INTERFACE_TEST!(),
        (access_point, "Password!2"),
        1000,
        (bool,),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    assert!(res.unwrap().0);
}

fn connect_to_known_access_point() {
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ListAccessPoints",
        NM_INTERFACE_TEST!(),
        (),
        1000,
        (Vec<AccessPoint>,),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    let mut access_point = res
        .expect("Failed to get access points")
        .0
        .first()
        .unwrap()
        .clone();
    // usually this would be saved, but the mock does not need to implement this.
    access_point.associated_connection = Path::from("/org/Xetibo/ReSet/Test/Connection/100");
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ConnectToKnownAccessPoint",
        NM_INTERFACE_TEST!(),
        (access_point,),
        1000,
        (bool,),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    assert!(res.unwrap().0);
}

#[tokio::test]
// tests connecting to a new access point with a *wrong* password
async fn test_connect_to_new_access_point_wrong_password() {
    setup();
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ListAccessPoints",
        NM_INTERFACE_TEST!(),
        (),
        1000,
        (Vec<AccessPoint>,),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    let access_point = res
        .expect("Failed to get access points")
        .0
        .first()
        .unwrap()
        .clone();
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ConnectToNewAccessPoint",
        NM_INTERFACE_TEST!(),
        (access_point, "wrong"),
        1000,
        (bool,),
    );
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
    assert!(!res.unwrap().0);
}

// #[tokio::test]
// async fn test_wireless_listener() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), ()>("StartNetworkListener", NETWORK_INTERFACE!(), ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     if let Err(_error) = res { panic!("connection failed: {}", (_error)); }
// }
//
// #[tokio::test]
// async fn test_bluetooth_listener() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(u32,), ()>(
//         "StartBluetoothListener",
//         BLUETOOTH_INTERFACE!(),
//         (5,),
//     );
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     if let Err(_error) = res { panic!("connection failed: {}", (_error)); }
// }
//

#[tokio::test]
#[serial]
async fn test_get_sinks() {
    setup();
    let res = call_session_dbus_method::<(), (Vec<Sink>,)>("ListSinks", AUDIO, ());
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
}

#[tokio::test]
#[serial]
async fn test_get_default_sink() {
    setup();
    let res = call_session_dbus_method::<(), (Sink,)>("GetDefaultSink", AUDIO, ());
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
}

#[tokio::test]
#[serial]
async fn test_get_default_source() {
    setup();
    let res = call_session_dbus_method::<(), (Source,)>("GetDefaultSource", AUDIO, ());
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
}

#[tokio::test]
#[serial]
async fn test_get_sources() {
    setup();
    let res = call_session_dbus_method::<(), (Vec<Source>,)>("ListSources", AUDIO, ());
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
}

#[tokio::test]
async fn test_get_input_streams() {
    setup();
    let res = call_session_dbus_method::<(), (Vec<InputStream>,)>("ListInputStreams", AUDIO, ());
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
}

#[tokio::test]
async fn test_get_output_streams() {
    setup();
    let res = call_session_dbus_method::<(), (Vec<OutputStream>,)>("ListOutputStreams", AUDIO, ());
    if let Err(_error) = res {
        panic!("connection failed: {}", (_error));
    }
}

#[tokio::test]
async fn test_plugins() {
    use re_set_lib::utils::plugin::plugin_tests;
    setup();
    unsafe {
        for plugin in BACKEND_PLUGINS.iter() {
            let name = (plugin.name)();
            let tests = (plugin.tests)();
            plugin_tests(name, tests);
        }
    }
}

// this is usually commencted out as it is used to test the mock dbus itself
// #[tokio::test]
// async fn mock_runner() {
//     setup();
//     thread::sleep(Duration::from_millis(60 * 60 * 1000));
// }
