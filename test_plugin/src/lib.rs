use std::{collections::HashMap, time::Duration};

use dbus::{blocking::Connection, Path};
use dbus_crossroads::Crossroads;
use re_set_lib::utils::{
    plugin::{PluginCapabilities, PluginData},
    variant::{Debug, TVariant, Variant},
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
    #[allow(clippy::unnecessary_cast)]
    // this cast is necessary -> u32 to i32 in explicit cast later on
    let test_data = (String::from("pingpang"), 10 as u32).into_variant();
    data.insert(String::from("pingpang"), test_data);
    let data = PluginData::new(data);
    cross.insert(
        Path::from("/org/Xetibo/ReSet/TestPlugin"),
        &interfaces,
        data,
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
    let res: Result<(String, u32), dbus::Error> =
        proxy.method_call("org.Xetibo.ReSet.TestPlugin", "Test", ());
    assert!(res.is_ok());
    let value = res.unwrap();
    assert_eq!(value.0, String::from("pingpang"));
    assert_eq!(value.1, 10);
}

pub fn setup_dbus_interface(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<PluginData> {
    cross.register("org.Xetibo.ReSet.TestPlugin", |c| {
        c.method(
            "Test",
            (),
            ("name", "age"),
            move |_, d: &mut PluginData, ()| {
                println!("Dbus function test called");
                let value = d.get_data_ref();
                let value = value
                    .get("pingpang")
                    .unwrap()
                    .to_value_cloned::<(String, u32)>()
                    .unwrap();
                Ok((value.0.clone(), value.1))
            },
        );
    })
}

#[derive(Debug, Clone)]
pub struct CustomPluginType {
    name: String,
    age: u32,
}

impl Debug for CustomPluginType {}

impl TVariant for CustomPluginType {
    fn into_variant(self) -> re_set_lib::utils::variant::Variant {
        Variant::new::<(String, u32)>((self.name.clone(), self.age))
    }

    fn value(&self) -> Box<dyn TVariant> {
        Box::new((self.name.clone(), self.age))
    }
}
