use std::{
    sync::{Arc, RwLock, RwLockWriteGuard},
    time::Duration,
};

use dbus::blocking::Connection;
use dbus_crossroads::IfaceBuilder;
use re_set_lib::{
    plug_assert, plug_assert_eq,
    utils::{
        plugin::{PluginCapabilities, PluginImplementation, PluginTestError, PluginTestFunc},
        plugin_setup::CrossWrapper,
        variant::{Debug, TVariant, Variant},
    },
};

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    println!("capabilities called");
    PluginCapabilities::new(vec!["test"], PluginImplementation::Backend)
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn name() -> String {
    println!("name called");
    String::from("testplugin")
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn dbus_interface(cross: Arc<RwLock<CrossWrapper>>) {
    println!("dbus interface called");
    let mut cross = cross.write().unwrap();
    let interface = setup_dbus_interface(&mut cross);
    cross.insert::<CustomPluginType>(
        "test",
        &[interface],
        CustomPluginType {
            name: "test person".to_string(),
            age: 10,
        },
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
#[allow(improper_ctypes_definitions)]
pub extern "C" fn backend_tests() -> Vec<PluginTestFunc> {
    println!("tests called");
    vec![PluginTestFunc::new(test1, "testconnection")]
}

fn test1() -> Result<(), PluginTestError> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(
        "org.Xetibo.ReSet.Daemon",
        "/org/Xetibo/ReSet/TestPlugin",
        Duration::from_millis(1000),
    );
    let res: Result<(String, u32), dbus::Error> =
        proxy.method_call("org.Xetibo.ReSet.TestPlugin", "Test", ());
    plug_assert!(res.is_ok())?;
    if res.is_err() {
        return Err(PluginTestError::new("didn't receive proper answer"));
    }

    let value = res.unwrap();
    plug_assert_eq!(value.0, "pingpang")?;
    plug_assert_eq!(value.1, 10)?;
    Ok(())
}

// pub fn setup_dbus_interface(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<PluginData> {
pub fn setup_dbus_interface(
    cross: &mut RwLockWriteGuard<CrossWrapper>,
) -> dbus_crossroads::IfaceToken<CustomPluginType> {
    cross.register::<CustomPluginType>(
        "org.Xetibo.ReSet.TestPlugin",
        |c: &mut IfaceBuilder<CustomPluginType>| {
            c.method(
                "Test",
                (),
                ("name", "age"),
                move |_, d: &mut CustomPluginType, ()| {
                    println!("Dbus function test called");
                    Ok((d.name.clone(), d.age))
                },
            );
        },
    )
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
