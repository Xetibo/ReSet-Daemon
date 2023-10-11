use std::time::Duration;

use dbus::{
    arg::{AppendAll, Get, ReadAll},
    blocking::Connection, Path,
};

pub fn call_system_dbus_method<I: AppendAll + 'static, O: ReadAll + 'static>(
    name: String,
    object: Path<'static>,
    function: String,
    proxy_name: String,
    params: I,
) -> Result<O, dbus::Error> {
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(name.as_str(), object, Duration::from_millis(1000));
    let result: Result<O, dbus::Error> = proxy.method_call(proxy_name.as_str(), function, params);
    result
}

pub fn get_system_dbus_property<I: AppendAll, O: for<'a> Get<'a> + 'static>(
    name: String,
    object: Path<'static>,
    interface: String,
    property: String,
) -> Result<O, dbus::Error> {
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(name.as_str(), object, Duration::from_millis(1000));
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

    let result: Result<O, dbus::Error> = proxy.get(interface.as_str(), property.as_str());
    result
}

pub fn call_session_dbus_method<
    I: AppendAll + Sync + Send + 'static,
    O: ReadAll + Sync + Send + 'static,
>(
    name: String,
    object: Path<'static>,
    function: String,
    proxy_name: String,
    params: I,
) -> Result<O, dbus::Error> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(name.as_str(), object, Duration::from_millis(1000));
    let result: Result<O, dbus::Error> = proxy.method_call(proxy_name.as_str(), function, params);
    result
}
