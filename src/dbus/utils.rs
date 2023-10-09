use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

use dbus::{
    arg::{self, AppendAll, Get, ReadAll, RefArg},
    blocking::Connection,
};

pub fn call_system_dbus_method<
    I: AppendAll + Sync + Send + 'static,
    O: ReadAll + Sync + Send + 'static,
>(
    name: String,
    object: String,
    function: String,
    proxy_name: String,
    params: I,
) -> JoinHandle<Result<O, dbus::Error>> {
    thread::spawn(move || {
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy(name.as_str(), object, Duration::from_millis(1000));
        let result: Result<O, dbus::Error> = proxy.method_call(proxy_name.as_str(), function, params);
        result
    })
}

pub fn get_system_dbus_property<
    I: AppendAll + Sync + Send + 'static,
    O: Sync + Send + for<'a> Get<'a> + 'static,
>(
    name: String,
    object: String,
    interface: String,
    property: String,
) -> JoinHandle<Result<O, dbus::Error>> {
    thread::spawn(move || {
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy(name.as_str(), object, Duration::from_millis(1000));
        use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

        let result: Result<O, dbus::Error> = proxy.get(interface.as_str(), property.as_str());
        result
    })
}

pub fn call_session_dbus_method<
    I: AppendAll + Sync + Send + 'static,
    O: ReadAll + Sync + Send + 'static,
>(
    name: String,
    object: String,
    function: String,
    proxy_name: String,
    params: I,
) -> JoinHandle<Result<O, dbus::Error>> {
    thread::spawn(move || {
        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy(name.as_str(), object, Duration::from_millis(1000));
        let result: Result<O, dbus::Error> = proxy.method_call(proxy_name.as_str(), function, params);
        result
    })
}
