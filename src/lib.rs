#[macro_use]
mod macros;
pub mod api;
mod audio;
mod bluetooth;
pub mod mock;
mod network;
pub mod plugin;
#[cfg(test)]
mod tests;
pub mod utils;

use re_set_lib::utils::config::CONFIG_STRING;
use re_set_lib::utils::flags::FLAGS;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::thread;
use std::{fs, future, process::exit, time::Duration};

use dbus::blocking::Connection;
use dbus::{channel::MatchingReceiver, message::MatchRule, Path};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;
use re_set_lib::utils::plugin_setup::{CrossWrapper, BACKEND_PLUGINS, PLUGIN_DIR};
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};
use re_set_lib::{ERROR, LOG};
use utils::{AudioRequest, AudioResponse, BASE};

use crate::{
    audio::audio_manager_dbus::setup_audio_manager,
    bluetooth::bluetooth_manager_dbus::setup_bluetooth_manager,
    network::network_manager_dbus::setup_wireless_manager, utils::DaemonData,
};

/// Version of the current package.
/// Use this to avoid version mismatch conflicts.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// # Running the daemon as a library function
///
/// Used as a standalone binary:
/// ```no_run
/// use reset_daemon::run_daemon;
///
/// #[tokio::main]
/// pub async fn main() {
///     run_daemon(None).await;
/// }
/// ```
///
/// The daemon will run to infinity, so it might be a good idea to put it into a different thread.
/// ```no_run
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicBool;
/// use reset_daemon::run_daemon;
/// let ready = Arc::new(AtomicBool::new(false));
/// tokio::task::spawn(run_daemon(Some(ready.clone())));
/// // wait for daemon to be ready
/// // your other code here...
/// ```
pub async fn run_daemon(ready: Option<Arc<AtomicBool>>) {
    for flag in FLAGS.0.iter() {
        // more configuration possible in the future
        match flag {
            re_set_lib::utils::flags::Flag::ConfigDir(config) => {
                LOG!("Use a different config file");
                unsafe {
                    *CONFIG_STRING = String::from(config);
                }
            }
            re_set_lib::utils::flags::Flag::PluginDir(path) => {
                LOG!("Use a different plugin directory");
                unsafe {
                    *PLUGIN_DIR = PathBuf::from(path);
                }
            }
            re_set_lib::utils::flags::Flag::Other(_flag) => {
                LOG!(format!(
                    "Custom flag {} with value {:#?}",
                    &_flag.0,
                    _flag.1.clone()
                ));
                // currently no other flags are supported or used, but might be used in plugins
            }
        }
    }
    create_log_file();

    LOG!("Running in debug mode");
    let res = connection::new_session_sync();
    if res.is_err() {
        return;
    }
    let (resource, conn) = res.unwrap();

    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    conn.request_name(BASE, false, true, false).await.unwrap();
    let mut cross = Crossroads::new();
    cross.set_async_support(Some((
        conn.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    let res = dbus_method!(
        NM_INTERFACE_BASE!(),
        Path::from(NM_PATH!()),
        "Introspect",
        "org.freedesktop.DBus.Introspectable",
        (),
        100,
        (),
    );
    let wifi_enabled = res.is_ok();
    let res = dbus_method!(
        BLUEZ_INTERFACE!(),
        "/",
        "Introspect",
        "org.freedesktop.DBus.Introspectable",
        (),
        100,
        (),
    );
    let bluetooth_enabled = res.is_ok();

    let mut features = Vec::new();
    let mut feature_strings = Vec::new();

    if wifi_enabled {
        features.push(setup_wireless_manager(&mut cross));
        feature_strings.push("WiFi");
        LOG!("WiFi feature started");
    }

    if bluetooth_enabled {
        features.push(setup_bluetooth_manager(&mut cross));
        // the agent is currently not implemented
        // features.push(setup_bluetooth_agent(&mut cross));
        feature_strings.push("Bluetooth");
        LOG!("Bluetooth feature started");
    }

    features.push(setup_audio_manager(&mut cross));
    feature_strings.push("Audio");

    unsafe {
        for plugin in BACKEND_PLUGINS.iter() {
            feature_strings.extend(plugin.capabilities.iter());
        }
    }

    let data = DaemonData::create(_handle, conn.clone());
    if data.is_err() {
        ERROR!(
            format!("{}", data.as_ref().err().unwrap().message),
            ErrorLevel::Critical
        );
        return;
    }
    let data = data.unwrap();

    if data
        .audio_listener_active
        .load(std::sync::atomic::Ordering::SeqCst)
        == false
    {
        let mut index = -1;
        for (i, feature) in feature_strings.iter().enumerate() {
            if *feature == "Audio" {
                index = i as i32;
            }
        }
        feature_strings.remove(index as usize);
    }

    features.push(setup_base(&mut cross, feature_strings));
    unsafe {
        thread::scope(|scope| {
            let wrapper = Arc::new(RwLock::new(CrossWrapper::new(&mut cross)));
            for plugin in BACKEND_PLUGINS.iter() {
                let wrapper_loop = wrapper.clone();
                scope.spawn(move || {
                    // allocate plugin specific things
                    (plugin.startup)();
                    // register and insert plugin interfaces
                    (plugin.data)(wrapper_loop);
                    let _name = (plugin.name)();
                    LOG!(format!("Loaded plugin: {}", _name));
                });
            }
        });
    }

    cross.insert(DBUS_PATH!(), &features, data);

    // register bluetooth agent before start
    // will be uncommented when agent is fully functional
    // {
    //     let data: &mut DaemonData = cross.data_mut(&Path::from(DBUS_PATH)).unwrap();
    //     if data.b_interface.current_adapter != Path::from("/") {
    //         // register bluetooth agent before listening to calls
    //         data.b_interface.register_agent();
    //     }
    // }
    //
    if let Some(ready) = ready {
        ready.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    conn.start_receive(
        MatchRule::new_method_call(),
        Box::new(move |msg, conn| {
            cross.handle_message(msg, conn).unwrap();
            true
        }),
    );

    future::pending::<()>().await;
    unreachable!()
}

fn create_log_file() {
    fs::File::create("/tmp/reset_daemon_log").expect("Could not create log file.");
}

fn setup_base(
    cross: &mut Crossroads,
    features: Vec<&'static str>,
) -> dbus_crossroads::IfaceToken<DaemonData> {
    cross.register(BASE, |c| {
        c.method("GetCapabilities", (), ("capabilities",), move |_, _, ()| {
            Ok((features.clone(),))
        });
        c.method("APIVersion", (), ("api-version",), move |_, _, ()| {
            // let the client handle the mismatch -> e.g. they decide if they want to keep using
            // the current daemon or not.
            Ok((VERSION,))
        });
        c.method(
            "RegisterClient",
            ("client_name",),
            ("result",),
            move |_, data: &mut DaemonData, (client_name,): (String,)| {
                data.clients.insert(client_name, data.clients.len());
                Ok((true,))
            },
        );
        c.method(
            "UnregisterClient",
            ("client_name",),
            ("result",),
            move |_, data: &mut DaemonData, (client_name,): (String,)| {
                data.clients.remove(&client_name);
                Ok((true,))
            },
        );
        c.method("Shutdown", (), (), move |_, data: &mut DaemonData, ()| {
            data.b_interface.unregister_agent();
            data.handle.abort();
            let _ = data.audio_sender.send(AudioRequest::StopListener);
            unsafe {
                for plugin in BACKEND_PLUGINS.iter() {
                    (plugin.shutdown)();
                }
            }
            exit(0);
            #[allow(unreachable_code)]
            Ok(())
        });
    })
}
