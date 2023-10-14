use std::time::Duration;

use dbus::{
    arg::{AppendAll, Get, ReadAll},
    blocking::Connection,
    Path,
};

pub fn call_system_dbus_method<I: AppendAll + 'static, O: ReadAll + 'static>(
    name: &str,
    object: Path<'static>,
    function: &str,
    proxy_name: &str,
    params: I,
) -> Result<O, dbus::Error> {
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(name, object, Duration::from_millis(1000));
    let result: Result<O, dbus::Error> = proxy.method_call(proxy_name, function, params);
    result
}

pub fn get_system_dbus_property<I: AppendAll, O: for<'a> Get<'a> + 'static>(
    name: &str,
    object: Path<'static>,
    interface: &str,
    property: &str,
) -> Result<O, dbus::Error> {
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(name, object, Duration::from_millis(1000));
    use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

    let result: Result<O, dbus::Error> = proxy.get(interface, property);
    result
}

pub fn call_session_dbus_method<
    I: AppendAll + Sync + Send + 'static,
    O: ReadAll + Sync + Send + 'static,
>(
    name: &str,
    object: Path<'static>,
    function: &str,
    proxy_name: &str,
    params: I,
) -> Result<O, dbus::Error> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(name, object, Duration::from_millis(1000));
    let result: Result<O, dbus::Error> = proxy.method_call(proxy_name, function, params);
    result
}
