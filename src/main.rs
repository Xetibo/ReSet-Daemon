mod audio;
mod network;
mod bluetooth;
mod lib;
mod utils;

use lib::run_daemon;

#[tokio::main]
pub async fn main() {
    run_daemon().await;
}

