use reset_daemon::{run_daemon, utils::Mode};

#[tokio::main]
pub async fn main() {
    run_daemon(Mode::Release).await;
}
