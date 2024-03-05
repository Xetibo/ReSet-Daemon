use super::mock_dbus::MockTestData;
use dbus_crossroads::Crossroads;
use re_set_lib::bluetooth::bluetooth_structures::{BluetoothAdapter, BluetoothDevice};

const MOCK_BLUETOOTH: &str = "org.Xetibo.ReSet.Test.Bluetooth";

pub fn mock_bluetooth_interface(
    cross: &mut Crossroads,
) -> dbus_crossroads::IfaceToken<MockTestData> {
    let token = cross.register(MOCK_BLUETOOTH, |c| {
        println!("pingpang bluetooth");
    });
    token
}

pub struct MockBluetoothData {
    adapters: Vec<BluetoothAdapter>,
    devices: Vec<BluetoothDevice>,
}

impl MockBluetoothData {
    pub fn new() -> Self {
        // TODO: add data for tests
        MockBluetoothData {
            adapters: Vec::new(),
            devices: Vec::new(),
        }
    }
}
