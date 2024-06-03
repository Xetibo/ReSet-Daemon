macro_rules! DBUS_PATH {
    () => {
        "/org/Xetibo/ReSet/Daemon"
    };
}

macro_rules! DBUS_PATH_TEST {
    () => {
        "/org/Xetibo/ReSet/Test"
    };
}

macro_rules! NETWORK_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Network"
    };
}

macro_rules! BLUETOOTH_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Bluetooth"
    };
}

#[cfg(test)]
macro_rules! BASE_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Daemon"
    };
}

macro_rules! BASE_TEST_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test"
    };
}

#[cfg(not(test))]
macro_rules! NM_INTERFACE_BASE {
    () => {
        "org.freedesktop.NetworkManager"
    };
}

#[cfg(test)]
macro_rules! NM_INTERFACE_BASE {
    () => {
        "org.Xetibo.ReSet.Test"
    };
}

#[cfg(not(test))]
macro_rules! NM_INTERFACE {
    () => {
        "org.freedesktop.NetworkManager"
    };
}

#[cfg(test)]
macro_rules! NM_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.NetworkManager"
    };
}

#[cfg(not(test))]
macro_rules! NM_SETTINGS_INTERFACE {
    () => {
        "org.freedesktop.NetworkManager.Settings"
    };
}

#[cfg(test)]
macro_rules! NM_SETTINGS_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.NetworkManager.Settings"
    };
}

#[cfg(test)]
macro_rules! NM_INTERFACE_TEST {
    () => {
        "org.Xetibo.ReSet.Network"
    };
}

#[cfg(not(test))]
macro_rules! NM_DEVICE_INTERFACE {
    () => {
        "org.freedesktop.NetworkManager.Device"
    };
}

#[cfg(test)]
macro_rules! NM_DEVICE_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.NetworkManager.Device"
    };
}

#[cfg(not(test))]
macro_rules! NM_DEVICE_WIRELESS_INTERFACE {
    () => {
        "org.freedesktop.NetworkManager.Device.Wireless"
    };
}

#[cfg(test)]
macro_rules! NM_DEVICE_WIRELESS_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.NetworkManager.Device"
    };
}

#[cfg(not(test))]
macro_rules! NM_ACCESS_POINT_INTERFACE {
    () => {
        "org.freedesktop.NetworkManager.AccessPoint"
    };
}

#[cfg(test)]
macro_rules! NM_ACCESS_POINT_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.NetworkManager.AccessPoint"
    };
}

#[cfg(not(test))]
macro_rules! NM_CONNECTION_INTERFACE {
    () => {
        "org.freedesktop.NetworkManager.Settings.Connection"
    };
}

#[cfg(test)]
macro_rules! NM_CONNECTION_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.NetworkManager.Connection"
    };
}

#[cfg(not(test))]
macro_rules! NM_ACTIVE_CONNECTION_INTERFACE {
    () => {
        "org.freedesktop.NetworkManager.Connection.Active"
    };
}

#[cfg(test)]
macro_rules! NM_ACTIVE_CONNECTION_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.NetworkManager.ActiveConnection"
    };
}

#[cfg(not(test))]
macro_rules! BLUEZ_INTERFACE {
    () => {
        "org.bluez"
    };
}

#[cfg(test)]
macro_rules! BLUEZ_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test"
    };
}

#[cfg(test)]
macro_rules! BLUEZ_ADAPTER_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.Bluez.Adapter"
    };
}

#[cfg(not(test))]
macro_rules! BLUEZ_ADAPTER_INTERFACE {
    () => {
        "org.bluez.Adapter1"
    };
}

#[cfg(test)]
macro_rules! BLUEZ_DEVICE_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.Bluez.Device"
    };
}

#[cfg(not(test))]
macro_rules! BLUEZ_DEVICE_INTERFACE {
    () => {
        "org.bluez.Device1"
    };
}

#[cfg(test)]
macro_rules! BLUEZ_AGENT_INTERFACE {
    () => {
        "org.Xetibo.ReSet.Test.Bluez.AgentManager"
    };
}

#[cfg(not(test))]
macro_rules! BLUEZ_AGENT_INTERFACE {
    () => {
        "org.bluez.AgentManager1"
    };
}

#[cfg(not(test))]
macro_rules! NM_PATH {
    () => {
        "/org/freedesktop/NetworkManager"
    };
}

#[cfg(test)]
macro_rules! NM_PATH {
    () => {
        "/org/Xetibo/ReSet/Test"
    };
}

#[cfg(not(test))]
macro_rules! NM_SETTINGS_PATH {
    () => {
        "/org/freedesktop/NetworkManager/Settings"
    };
}

#[cfg(test)]
macro_rules! NM_SETTINGS_PATH {
    () => {
        "/org/Xetibo/ReSet/Test"
    };
}

#[cfg(not(test))]
macro_rules! NM_DEVICES_PATH {
    () => {
        "/org/freedesktop/NetworkManager/Devices"
    };
}

#[cfg(test)]
macro_rules! NM_DEVICES_PATH {
    () => {
        "/org/Xetibo/ReSet/Test/Devices"
    };
}

#[cfg(not(test))]
macro_rules! NM_ACCESS_POINT_PATH {
    () => {
        "/org/freedesktop/NetworkManager/AccessPoint"
    };
}

#[cfg(test)]
macro_rules! NM_ACCESS_POINT_PATH {
    () => {
        "/org/Xetibo/ReSet/Test/Devices"
    };
}

#[cfg(not(test))]
macro_rules! NM_ACTIVE_CONNECTION_PATH {
    () => {
        "/org/freedesktop/NetworkManager/ActiveConnection/"
    };
}

#[cfg(test)]
macro_rules! NM_ACTIVE_CONNECTION_PATH {
    () => {
        "/org/Xetibo/ReSet/Test"
    };
}

#[cfg(not(test))]
macro_rules! BLUEZ_PATH {
    () => {
        "/org/bluez"
    };
}

#[cfg(test)]
macro_rules! BLUEZ_PATH {
    () => {
        "/org/Xetibo/ReSet/Test/Bluez"
    };
}

#[cfg(not(test))]
macro_rules! BLUEZ_CONTAINS_PATH {
    () => {
        "bluez"
    };
}

#[cfg(test)]
macro_rules! BLUEZ_CONTAINS_PATH {
    () => {
        "Xetibo"
    };
}

#[cfg(not(test))]
macro_rules! dbus_method {
    (
    $name:expr,
    $object:expr,
    $function:expr,
    $proxy_name:expr,
    $params:expr,
    $time:expr,
    $output:ty,
) => {{
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy($name, $object, Duration::from_millis($time));
        let result: Result<$output, dbus::Error> =
            proxy.method_call($proxy_name, $function, $params);
        result
    }};
}

#[cfg(test)]
macro_rules! dbus_method {
    (
    $name:expr,
    $object:expr,
    $function:expr,
    $proxy_name:expr,
    $params:expr,
    $time:expr,
    $output:ty,
) => {{
        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy($name, $object, Duration::from_millis($time));
        let result: Result<$output, dbus::Error> =
            proxy.method_call($proxy_name, $function, $params);
        result
    }};
}

#[cfg(not(test))]
macro_rules! get_dbus_property {
    (
    $name:expr,
    $object:expr,
    $interface:expr,
    $property:expr,
    $output:ty,
) => {{
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy($name, $object, Duration::from_millis(1000));
        use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

        let result: Result<$output, dbus::Error> = proxy.get($interface, $property);
        result
    }};
}

#[cfg(test)]
macro_rules! get_dbus_property {
    (
    $name:expr,
    $object:expr,
    $interface:expr,
    $property:expr,
    $output:ty,
) => {{
        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy($name, $object, Duration::from_millis(1000));
        use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

        let result: Result<$output, dbus::Error> = proxy.get($interface, $property);
        result
    }};
}

#[cfg(not(test))]
macro_rules! set_dbus_property {
    (
    $name:expr,
    $object:expr,
    $interface:expr,
    $property:expr,
    $value:expr,
) => {{
        let conn = Connection::new_system().unwrap();
        let proxy = conn.with_proxy($name, $object, Duration::from_millis(1000));
        use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

        let result: Result<(), dbus::Error> = proxy.set($interface, $property, $value);
        result
    }};
}

#[cfg(test)]
macro_rules! set_dbus_property {
    (
    $name:expr,
    $object:expr,
    $interface:expr,
    $property:expr,
    $value:expr,
) => {{
        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy($name, $object, Duration::from_millis(1000));
        use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

        let result: Result<(), dbus::Error> = proxy.set($interface, $property, $value);
        result
    }};
}

#[allow(unused_macros)]
macro_rules! dbus_connection {
    () => {
        Connection::new_system().unwrap()
    };
}

#[cfg(test)]
macro_rules! dbus_connection {
    () => {
        Connection::new_session().unwrap()
    };
}
