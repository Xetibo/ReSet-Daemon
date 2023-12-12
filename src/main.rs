use reset_daemon::run_daemon;

#[tokio::main]
pub async fn main() {
    run_daemon(false, "org.Xetibo.ReSet.Daemon").await;
}
