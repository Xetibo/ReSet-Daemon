use crate::mock::mock_dbus::start_mock_implementation_server;
#[allow(unused_imports)]
use crate::{
    run_daemon,
    utils::{AUDIO, BASE},
};
use dbus::{
    arg::{AppendAll, ReadAll},
    blocking::Connection,
};

#[allow(unused_imports)]
use re_set_lib::audio::audio_structures::{InputStream, OutputStream, Source};

use std::{
    hint,
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        Once,
    },
};
#[allow(unused_imports)]
use std::{thread, time::Duration};
use tokio::runtime;

#[allow(dead_code)]
static START_DAEMON: Once = Once::new();
static COUNTER: AtomicU16 = AtomicU16::new(0);
static READY: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
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
            rt.spawn(run_daemon());
            while COUNTER.load(Ordering::SeqCst) != 0 {
                hint::spin_loop();
            }
            rt.shutdown_background();
        });
    };
}

// #[tokio::test]
// async fn test_check() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     println!("{}", DBUS_PATH!());
//     let res = call_session_dbus_method::<(), ()>("Check", BASE, ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
// }

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
    assert!(res.is_ok());
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
    assert!(res.is_ok());
    assert!(!res.unwrap().0.is_empty());
}

#[tokio::test]
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
    assert!(res.is_ok());
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
    assert!(res.is_ok());
    assert_eq!(res.unwrap().0.len(), 1);
}

//
// #[tokio::test]
// async fn test_audio_listener() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), ()>("StartAudioListener", AUDIO, ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
// }
//
// #[tokio::test]
// async fn test_wireless_listener() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), ()>("StartNetworkListener", NETWORK_INTERFACE!(), ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
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
//     assert!(res.is_ok());
// }
//
// #[tokio::test]
// async fn test_get_sinks() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), (Vec<Sink>,)>("ListSinks", AUDIO, ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
// }
//
// #[tokio::test]
// async fn test_get_default_sink() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), (Sink,)>("GetDefaultSink", AUDIO, ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
// }
//
// #[tokio::test]
// async fn test_get_default_source() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), (Source,)>("GetDefaultSource", AUDIO, ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
// }
//
// #[tokio::test]
// async fn test_get_sources() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), (Vec<Source>,)>("ListSources", AUDIO, ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
// }
//
// #[tokio::test]
// async fn test_get_input_streams() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), (Vec<InputStream>,)>("ListInputStreams", AUDIO, ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
// }
//
// #[tokio::test]
// async fn test_get_output_streams() {
//     setup();
//     thread::sleep(Duration::from_millis(1000));
//     let res = call_session_dbus_method::<(), (Vec<OutputStream>,)>("ListOutputStreams", AUDIO, ());
//     COUNTER.fetch_sub(1, Ordering::SeqCst);
//     assert!(res.is_ok());
// }
