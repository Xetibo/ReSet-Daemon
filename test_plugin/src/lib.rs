use std::collections::HashMap;

use dbus::Path;
use re_set_lib::utils::plugin::{Plugin, PluginCapabilities, PluginData};

#[no_mangle]
pub extern "C" fn capabilities() -> PluginCapabilities {
    println!("capabilities called");
    PluginCapabilities::new(vec![String::from("test")])
}

#[no_mangle]
pub extern "C" fn dbus_interface() -> Plugin {
    println!("dbus interface called");
    Plugin::new(Path::from("/asldkfj"), Vec::new(), PluginData::new(HashMap::new()))
}

#[no_mangle]
pub extern "C" fn startup() {
    println!("startup called");
}

#[no_mangle]
pub extern "C" fn shutdown() {
    println!("shutdown called");
}

#[no_mangle]
pub extern "C" fn tests() {
    println!("tests called");
}
