mod audio;
mod network;
mod bluetooth;
mod reset_dbus;
mod utils;

use reset_dbus::run_daemon;

#[tokio::main]
pub async fn main() {
    run_daemon().await;
}

