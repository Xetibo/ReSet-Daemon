use dbus_crossroads::Crossroads;
use dbus_tokio::connection;
use std::future;

use super::mock_dbus::MockNetworkData;

const MOCK_WIRELESS: &'static str = "MOCKnetwork";

pub fn mock_network_interface(
    cross: &mut Crossroads,
) -> dbus_crossroads::IfaceToken<MockNetworkData> {
    let token = cross.register(MOCK_WIRELESS, |c| {
        println!("pingpang");
    });
    token
}


