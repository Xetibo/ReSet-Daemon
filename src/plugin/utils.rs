use std::{fs::create_dir, io::ErrorKind};

use dbus_crossroads::Crossroads;
use once_cell::sync::Lazy;
use re_set_lib::{
    create_config,
    utils::{macros::ErrorLevel, plugin::PluginCapabilities},
    write_log_to_file, ERROR,
};

use crate::PluginFunctions;

pub static mut PLUGINS: Lazy<Vec<PluginFunctions>> = Lazy::new(|| {
    SETUP_LIBS();
    SETUP_PLUGINS()
});
static mut LIBS: Vec<libloading::Library> = Vec::new();

static SETUP_LIBS: fn() = || {
    let config = create_config("Xetibo", "ReSet").expect("Could not create config directory");
    let plugin_dir = create_dir(config.join("plugins"));
    let plugin_dir = if let Err(error) = plugin_dir {
        if error.kind() != ErrorKind::AlreadyExists {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Failed to read plugin directory",
                ErrorLevel::Critical
            );
            None
        } else {
            Some(config.join("plugins"))
        }
    } else {
        Some(config.join("plugins"))
    };
    if let Some(plugin_dir) = plugin_dir {
        let plugin_dir = plugin_dir.read_dir().expect("what");
        plugin_dir.for_each(|plugin| {
            if let Ok(file) = plugin {
                unsafe {
                    LIBS.push(
                        libloading::Library::new(file.path()).expect("Could not open plugin."),
                    );
                }
            }
        });
    }
};

static SETUP_PLUGINS: fn() -> Vec<PluginFunctions> = || -> Vec<PluginFunctions> {
    let mut plugins = Vec::new();
    unsafe {
        for lib in LIBS.iter() {
            let dbus_interface: Result<
                libloading::Symbol<unsafe extern "C" fn(&mut Crossroads)>, // -> Plugin>,
                libloading::Error,
            > = lib.get(b"dbus_interface");
            let startup: Result<
                libloading::Symbol<unsafe extern "C" fn() -> ()>,
                libloading::Error,
            > = lib.get(b"backend_startup");
            let shutdown: Result<
                libloading::Symbol<unsafe extern "C" fn() -> ()>,
                libloading::Error,
            > = lib.get(b"backend_shutdown");
            let capabilities: Result<
                libloading::Symbol<unsafe extern "C" fn() -> PluginCapabilities>,
                libloading::Error,
            > = lib.get(b"capabilities");
            let tests: Result<libloading::Symbol<unsafe extern "C" fn() -> ()>, libloading::Error> =
                lib.get(b"backend_tests");
            if let (Ok(dbus_interface), Ok(startup), Ok(shutdown), Ok(capabilities), Ok(tests)) =
                (dbus_interface, startup, shutdown, capabilities, tests)
            {
                plugins.push(PluginFunctions::new(
                    startup,
                    shutdown,
                    capabilities,
                    dbus_interface,
                    tests,
                ));
            } else {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Failed to load plugin",
                    ErrorLevel::Critical
                );
            }
        }
    }
    plugins
};
