// somehow clippy doesn't recognize the tests properly, which leads to wrongly placed "unused
// imports"
use crate::{mock::mock_dbus::start_mock_implementation_server, BACKEND_PLUGINS};
use crate::{run_daemon, utils::AUDIO};
use dbus::{
    arg::{AppendAll, ReadAll},
    blocking::Connection,
    Path,
};

use serial_test::serial;

use re_set_lib::audio::audio_structures::Sink;
use re_set_lib::audio::audio_structures::{InputStream, OutputStream, Source};
use re_set_lib::bluetooth::bluetooth_structures::BluetoothDevice;
use re_set_lib::network::network_structures::AccessPoint;

use std::{
    hint,
    sync::atomic::{AtomicBool, AtomicU16, Ordering},
};
use std::{thread, time::Duration};
use tokio::runtime;

static COUNTER: AtomicU16 = AtomicU16::new(0);
static READY: AtomicBool = AtomicBool::new(false);

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

#[allow(dead_code)]
fn setup() {
    if COUNTER.fetch_add(1, Ordering::SeqCst) < 1 {
        thread::spawn(|| {
            let rt2 = runtime::Runtime::new().expect("Failed to create runtime");
            rt2.spawn(start_mock_implementation_server(&READY));
            while !READY.load(Ordering::SeqCst) {
                hint::spin_loop();
            }
            let rt = runtime::Runtime::new().expect("Failed to create runtime");
            rt.spawn(run_daemon(None));
            while COUNTER.load(Ordering::SeqCst) != 0 {
                hint::spin_loop();
            }
            rt.shutdown_background();
        });
    };
}

#[tokio::test]
// tests fetching bluetooth devices
async fn test_bluetooth_get_devices() {
    setup();
    while !READY.load(Ordering::SeqCst) {
        hint::spin_loop();
    }
    thread::sleep(Duration::from_millis(1000));
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "StartBluetoothScan",
        BLUETOOTH_INTERFACE!(),
        (),
        1000,
        (),
    );
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
    thread::sleep(Duration::from_millis(1000));
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "GetBluetoothDevices",
        BLUETOOTH_INTERFACE!(),
        (),
        1000,
        (Vec<BluetoothDevice>,),
    );
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
    assert!(!res.unwrap().0.is_empty());
}

#[tokio::test]
// tests the existance of the mock implementation
async fn test_mock_connection() {
    setup();
    while !READY.load(Ordering::SeqCst) {
        hint::spin_loop();
    }
    let conn = Connection::new_session();
    let conn = conn.unwrap();
    let proxy = conn.with_proxy(
        BASE_TEST_INTERFACE!(),
        DBUS_PATH_TEST!(),
        Duration::from_millis(2000),
    );
    let res: Result<(), dbus::Error> = proxy.method_call(NM_INTERFACE!(), "Test", ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
}

#[tokio::test]
// tests receiving a list of connections through both the mock implementation and the ReSet Daemon
async fn test_list_connections() {
    setup();
    thread::sleep(Duration::from_millis(2000));
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
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
    assert!(!res.unwrap().0.is_empty());
}

#[tokio::test]
#[serial]
// tests adding and removing an access point
async fn test_add_access_point_event() {
    setup();
    thread::sleep(Duration::from_millis(2000));
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
    if let Err(error) = res {
        panic!("connection failed: {}", error);
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
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
    assert_eq!(res.unwrap().0.len(), 1);
}

#[tokio::test]
#[serial]
// tests connecting to a new access point with a password
async fn test_connect_to_new_access_point() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    connect_to_new_access_point();
    thread::sleep(Duration::from_millis(1000));
    connect_to_known_access_point();
    COUNTER.fetch_sub(1, Ordering::SeqCst);
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
    if let Err(error) = res {
        panic!("connection failed: {}", error);
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
    if let Err(error) = res {
        panic!("connection failed: {}", error);
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
    if let Err(error) = res {
        panic!("connection failed: {}", error);
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
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
    assert!(res.unwrap().0);
}

#[tokio::test]
// tests connecting to a new access point with a *wrong* password
async fn test_connect_to_new_access_point_wrong_password() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res = dbus_method!(
        BASE_INTERFACE!(),
        DBUS_PATH!(),
        "ListAccessPoints",
        NM_INTERFACE_TEST!(),
        (),
        1000,
        (Vec<AccessPoint>,),
    );
    if let Err(error) = res {
        panic!("connection failed: {}", error);
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
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
    assert!(!res.unwrap().0);
}

// #[tokio::test]
// async fn test_wireless_listener() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), ()>("StartNetworkListener", NETWORK_INTERFACE!(), ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     if let Err(error) = res { panic!("connection failed: {}", error); }
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
//     if let Err(error) = res { panic!("connection failed: {}", error); }
// }
//

#[tokio::test]
async fn test_get_sinks() {
    setup();
    thread::sleep(Duration::from_millis(2000));
    let res = call_session_dbus_method::<(), (Vec<Sink>,)>("ListSinks", AUDIO, ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
}

#[tokio::test]
async fn test_get_default_sink() {
    setup();
    thread::sleep(Duration::from_millis(2000));
    let res = call_session_dbus_method::<(), (Sink,)>("GetDefaultSink", AUDIO, ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
}

#[tokio::test]
async fn test_get_default_source() {
    setup();
    thread::sleep(Duration::from_millis(2000));
    let res = call_session_dbus_method::<(), (Source,)>("GetDefaultSource", AUDIO, ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
}

#[tokio::test]
async fn test_get_sources() {
    setup();
    thread::sleep(Duration::from_millis(2000));
    let res = call_session_dbus_method::<(), (Vec<Source>,)>("ListSources", AUDIO, ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
}

#[tokio::test]
async fn test_get_input_streams() {
    setup();
    thread::sleep(Duration::from_millis(2000));
    let res = call_session_dbus_method::<(), (Vec<InputStream>,)>("ListInputStreams", AUDIO, ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
}

#[tokio::test]
async fn test_get_output_streams() {
    setup();
    thread::sleep(Duration::from_millis(2000));
    let res = call_session_dbus_method::<(), (Vec<OutputStream>,)>("ListOutputStreams", AUDIO, ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    if let Err(error) = res {
        panic!("connection failed: {}", error);
    }
}

#[tokio::test]
#[cfg(test)]
async fn test_plugins() {
    use re_set_lib::utils::plugin::plugin_tests;
    setup();
    thread::sleep(Duration::from_millis(2000));
    unsafe {
        for plugin in BACKEND_PLUGINS.iter() {
            let name = (plugin.name)();
            let tests = (plugin.tests)();
            plugin_tests(name, tests);
        }
    }
    COUNTER.fetch_sub(1, Ordering::SeqCst);
}

// this is usually commencted out as it is used to test the mock dbus itself
// #[tokio::test]
// async fn mock_runner() {
//     setup();
//     thread::sleep(Duration::from_millis(60 * 60 * 1000));
// }
