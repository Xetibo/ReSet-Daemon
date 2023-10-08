use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

use dbus::{
    arg::{AppendAll, ReadAll},
    blocking::Connection,
};

pub fn call_system_dbus_method<
    I: AppendAll + Sync + Send + 'static,
    O: ReadAll + Sync + Send + 'static,
>(
    name: &'static str,
    object: &'static str,
    function: &'static str,
    params: I,
) -> JoinHandle<Result<O, dbus::Error>> {
    thread::spawn(move || {
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy(name, object, Duration::from_millis(1000));
        let result: Result<O, dbus::Error> = proxy.method_call(name, function, params);
        result
    })
}

pub fn call_session_dbus_method<
    I: AppendAll + Sync + Send + 'static,
    O: ReadAll + Sync + Send + 'static,
>(
    name: &'static str,
    object: &'static str,
    function: &'static str,
    params: I,
) -> JoinHandle<Result<O, dbus::Error>> {
    thread::spawn(move || {
        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy(name, object, Duration::from_millis(1000));
        let result: Result<O, dbus::Error> = proxy.method_call(name, function, params);
        result
    })
}
