use dbus::blocking::Connection;
use std::{any::Any, sync::Arc};

pub mod audio;
pub mod bluetooth;
pub mod network;
pub mod reset_dbus;
mod utils;

pub trait Data: Any + Sync + Send {}

pub struct ReSetData {
    capabilities: Vec<&'static str>,
    // networks: Vec<ReSet_Lib::Network>,
    networks: Vec<String>,
    // audio: Vec<ReSet_Lib::Network>,
    // bluetooth: Vec<ReSet_Lib::Network>,
    plugin_data: Vec<Arc<dyn Data>>,
}

impl ReSetData {
    pub fn add_capability(&mut self, name: &'static str) {
        self.capabilities.push(name);
    }
}

pub struct ReSetDaemon {
    data: Arc<ReSetData>,
}

impl ReSetDaemon {
    pub fn create() -> Self {
        Self {
            data: Arc::new(ReSetData {
                capabilities: vec!["Network", "Bluetooth", "Audio"],
                plugin_data: Vec::new(),
                networks: Vec::new(),
            }),
        }
    }

    /// GetCapabilities input: none, output: Vec<String>
    /// Returns the capabilities of the current daemon, the base fucntions are Network,
    /// Audio and Bluetooth, anything additional will be provided via plugins.
    ///
    /// GetNetworks input: none, output: Vec<String>
    /// Returns all available networks.
    pub fn run(&self) {
        let connection = Connection::new_session().unwrap();
        connection
            .request_name("org.xetibo.ReSet", true, true, false)
            .unwrap();
        let mut cr = dbus_crossroads::Crossroads::new();
        let token = cr.register("org.xetibo.ReSet", |connection| {
            connection.method(
                "GetCapabilities",
                (),
                ("Capabilities",),
                |_, data: &mut Arc<ReSetData>, ()| Ok((data.capabilities.clone(),)),
            );
            connection.method(
                "GetNetworks",
                (),
                ("Networks",),
                |_, data: &mut Arc<ReSetData>, ()| Ok((data.networks.clone(),)),
            );
        });
        cr.insert("/org/xetibo/ReSet", &[token], self.data.clone());
        cr.serve(&connection).unwrap();
    }
}
