use std::{collections::HashMap, time::Duration};

use dbus::{blocking::Connection, Path};
use dbus_crossroads::Crossroads;
use re_set_lib::utils::{
    plugin::{PluginCapabilities, PluginData},
    variant::TVariant,
};

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    println!("capabilities called");
    PluginCapabilities::new(vec!["test"])
}

#[no_mangle]
pub extern "C" fn dbus_interface(cross: &mut Crossroads) {
    println!("dbus interface called");
    let mut interfaces = Vec::new();
    let interface = setup_dbus_interface(cross);
    interfaces.push(interface);
    let mut data = HashMap::new();
    data.insert(String::from("pingpang"), 10.into_variant());
    cross.insert(
        Path::from("/org/Xetibo/ReSet/TestPlugin"),
        &interfaces,
        PluginData::new(data),
    );
}

#[no_mangle]
pub extern "C" fn backend_startup() {
    println!("startup called");
}

#[no_mangle]
pub extern "C" fn backend_shutdown() {
    println!("shutdown called");
}

#[no_mangle]
pub extern "C" fn backend_tests() {
    println!("tests called");
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(
        "org.Xetibo.ReSet.Daemon",
        "/org/Xetibo/ReSet/TestPlugin",
        Duration::from_millis(1000),
    );
    let res: Result<(i32,), dbus::Error> =
        proxy.method_call("org.Xetibo.ReSet.TestPlugin", "Test", ());
    assert!(res.is_ok());
    assert_eq!(res.unwrap().0, 10);
}

pub fn setup_dbus_interface(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<PluginData> {
    cross.register("org.Xetibo.ReSet.TestPlugin", |c| {
        c.method("Test", (), ("test",), move |_, d: &mut PluginData, ()| {
            println!("Dbus function test called");
            Ok((d
                .get_data()
                .get(&String::from("pingpang"))
                .unwrap()
                .to_value::<i32>()
                .unwrap(),))
        });
    })
}
