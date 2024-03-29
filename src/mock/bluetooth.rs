use std::{thread, time::Duration};

use dbus::Path;
use dbus_crossroads::Crossroads;

pub fn mock_bluetooth_adapter_interface(
    cross: &mut Crossroads,
) -> dbus_crossroads::IfaceToken<MockBluetoothAdapterData> {
    cross.register(BLUEZ_ADAPTER_INTERFACE!(), |c| {
        c.property("Discoverable")
            .set(|_, data: &mut MockBluetoothAdapterData, new_value| {
                data.discoverable = new_value;
                Ok(Some(new_value))
            })
            .get(|_, data: &mut MockBluetoothAdapterData| Ok(data.discoverable));
        c.property("Pairable")
            .set(|_, data: &mut MockBluetoothAdapterData, new_value| {
                data.pairable = new_value;
                Ok(Some(new_value))
            })
            .get(|_, data: &mut MockBluetoothAdapterData| Ok(data.pairable));
        c.property("Powered")
            .set(|_, data: &mut MockBluetoothAdapterData, new_value| {
                data.powered = new_value;
                Ok(Some(new_value))
            })
            .get(|_, data: &mut MockBluetoothAdapterData| Ok(data.powered));
        c.property("Alias")
            .get(|_, data: &mut MockBluetoothAdapterData| Ok(data.alias.clone()));
        c.method_with_cr_async("StartDiscovery", (), (), move |mut ctx, cross, ()| {
            let data: &mut MockBluetoothAdapterData = cross.data_mut(ctx.path()).unwrap();
            let adapter_path = data.adapter_path.clone();
            let interface = data.device_interface;
            create_mock_bluetooth_device(
                cross,
                interface,
                &Path::from(BLUEZ_PATH!().to_string() + "/hci0/Device1"),
                adapter_path,
            );
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async("StopDiscovery", (), (), move |mut ctx, cross, ()| {
            cross.remove::<dbus_crossroads::IfaceToken<MockBluetoothAdapterData>>(&Path::from(
                BLUEZ_PATH!().to_string() + "/hci0/Device1",
            ));
            async move { ctx.reply(Ok(())) }
        });
    })
}

pub fn mock_bluetooth_device_interface(
    cross: &mut Crossroads,
) -> dbus_crossroads::IfaceToken<MockBluetoothDeviceData> {
    cross.register(BLUEZ_DEVICE_INTERFACE!(), |c| {
        c.property("Connected")
            .get(|_, data: &mut MockBluetoothDeviceData| Ok(data.connected));
        c.property("Trusted")
            .get(|_, _: &mut MockBluetoothDeviceData| Ok(false));
        c.property("Bonded")
            .get(|_, _: &mut MockBluetoothDeviceData| Ok(false));
        c.property("Blocked")
            .get(|_, _: &mut MockBluetoothDeviceData| Ok(false));
        c.property("Paired")
            .get(|_, _: &mut MockBluetoothDeviceData| Ok(false));
        c.property("Adapter")
            .get(|_, data: &mut MockBluetoothDeviceData| Ok(data.adapter.clone()));
        c.method_with_cr_async("Connect", (), (), move |mut ctx, cross, ()| {
            let data: &mut MockBluetoothDeviceData = cross.data_mut(ctx.path()).unwrap();
            data.connected = true;
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async("Pair", (), (), move |mut ctx, cross, ()| {
            let data: &mut MockBluetoothDeviceData = cross.data_mut(ctx.path()).unwrap();
            data.connected = true;
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async("Disconnect", (), (), move |mut ctx, cross, ()| {
            let data: &mut MockBluetoothDeviceData = cross.data_mut(ctx.path()).unwrap();
            data.connected = false;
            thread::sleep(Duration::from_millis(1000));
            async move { ctx.reply(Ok(())) }
        });
    })
}

fn create_mock_bluetooth_device(
    cross: &mut Crossroads,
    interface: dbus_crossroads::IfaceToken<MockBluetoothDeviceData>,
    path: &Path<'static>,
    adapter_path: Path<'static>,
) {
    cross.insert(
        path.clone(),
        &[interface],
        MockBluetoothDeviceData::new(adapter_path),
    );
}

fn create_mock_bluetooth_adapter(
    cross: &mut Crossroads,
    interface: dbus_crossroads::IfaceToken<MockBluetoothAdapterData>,
    device_interface: dbus_crossroads::IfaceToken<MockBluetoothDeviceData>,
    path: &Path<'static>,
) {
    cross.insert(
        path.clone(),
        &[interface],
        MockBluetoothAdapterData::new(path.clone(), device_interface),
    );
}

#[allow(dead_code)]
pub struct MockBluetooth {
    pub adapter_interface: dbus_crossroads::IfaceToken<MockBluetoothAdapterData>,
    pub data: MockBluetoothData,
}

impl MockBluetooth {
    pub fn new(cross: &mut Crossroads) -> Self {
        let device_interface = mock_bluetooth_device_interface(cross);
        let adapter_interface = mock_bluetooth_adapter_interface(cross);
        let adapter_path = Path::from(BLUEZ_PATH!().to_string() + "/hci0");
        create_mock_bluetooth_adapter(
            cross,
            adapter_interface,
            device_interface,
            &adapter_path.clone(),
        );
        MockBluetooth {
            adapter_interface,
            data: MockBluetoothData::new(adapter_path, device_interface),
        }
    }
}

pub struct MockBluetoothData {
    pub adapter_data: MockBluetoothAdapterData,
    pub device_data: MockBluetoothDeviceData,
}

impl MockBluetoothData {
    pub fn new(
        adapter_path: Path<'static>,
        device_interface: dbus_crossroads::IfaceToken<MockBluetoothDeviceData>,
    ) -> Self {
        Self {
            adapter_data: MockBluetoothAdapterData::new(adapter_path.clone(), device_interface),
            device_data: MockBluetoothDeviceData::new(adapter_path),
        }
    }
}

pub struct MockBluetoothDeviceData {
    connected: bool,
    adapter: Path<'static>,
}

impl MockBluetoothDeviceData {
    pub fn new(adapter: Path<'static>) -> Self {
        Self {
            connected: false,
            adapter,
        }
    }
}

pub struct MockBluetoothAdapterData {
    pub adapter_path: Path<'static>,
    pub device_interface: dbus_crossroads::IfaceToken<MockBluetoothDeviceData>,
    pub discoverable: bool,
    pub pairable: bool,
    pub powered: bool,
    pub alias: String,
}

impl MockBluetoothAdapterData {
    pub fn new(
        adapter_path: Path<'static>,
        device_interface: dbus_crossroads::IfaceToken<MockBluetoothDeviceData>,
    ) -> Self {
        Self {
            adapter_path,
            device_interface,
            discoverable: false,
            pairable: false,
            powered: false,
            alias: String::from("test_adapter"),
        }
    }
}
