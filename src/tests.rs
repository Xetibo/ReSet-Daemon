use crate::run_daemon;
use dbus::{
    arg::{AppendAll, ReadAll},
    blocking::Connection,
};
use std::{
    hint,
    sync::{
        atomic::{AtomicU16, Ordering},
        Once,
    },
};
#[allow(unused_imports)]
use std::{thread, time::Duration};
use tokio::runtime;
#[cfg(test)]
use ReSet_Lib::audio::audio::Sink;

#[allow(dead_code)]
static START_DAEMON: Once = Once::new();
static COUNTER: AtomicU16 = AtomicU16::new(0);

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
    if conn.is_err() {
        println!("wat");
    }
    let conn = conn.unwrap();
    let proxy = conn.with_proxy(
        "org.Xetibo.ReSetDaemon",
        "/org/Xetibo/ReSetDaemon",
        Duration::from_millis(2000),
    );
    let result: Result<O, dbus::Error> = proxy.method_call(proxy_name, function, params);
    result
}

#[allow(dead_code)]
fn setup() {
    if COUNTER.fetch_add(1, Ordering::SeqCst) < 1 {
        thread::spawn(|| {
            let rt = runtime::Runtime::new().expect("Failed to create runtime");
            rt.spawn(run_daemon());
            while COUNTER.load(Ordering::SeqCst) != 0 {
                hint::spin_loop();
            }
            rt.shutdown_background();
        });
    };
}

#[tokio::test]
async fn test_check() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res = call_session_dbus_method::<(), ()>("Check", "org.Xetibo.ReSetDaemon", ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_audio_listener() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res = call_session_dbus_method::<(), ()>("StartAudioListener", "org.Xetibo.ReSetAudio", ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_wireless_listener() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res =
        call_session_dbus_method::<(), ()>("StartNetworkListener", "org.Xetibo.ReSetWireless", ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_bluetooth_listener() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res = call_session_dbus_method::<(u32,), ()>(
        "StartBluetoothListener",
        "org.Xetibo.ReSetBluetooth",
        (5,),
    );
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_get_sinks() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res =
        call_session_dbus_method::<(), (Vec<Sink>,)>("ListSinks", "org.Xetibo.ReSetAudio", ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_get_sources() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res =
        call_session_dbus_method::<(), (Vec<Sink>,)>("ListSources", "org.Xetibo.ReSetAudio", ());
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_get_input_streams() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res = call_session_dbus_method::<(), (Vec<Sink>,)>(
        "ListInputStreams",
        "org.Xetibo.ReSetAudio",
        (),
    );
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_get_output_streams() {
    setup();
    thread::sleep(Duration::from_millis(1000));
    let res = call_session_dbus_method::<(), (Vec<Sink>,)>(
        "ListOutputStreams",
        "org.Xetibo.ReSetAudio",
        (),
    );
    COUNTER.fetch_sub(1, Ordering::SeqCst);
    assert!(res.is_ok());
}
