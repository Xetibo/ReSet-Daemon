use std::sync::Arc;

use crate::network::network_manager::Device;
use dbus::{
    arg::PropMap, channel::Sender, nonblock::SyncConnection, Message, Path,
};
use dbus_crossroads::Crossroads;

use super::mock_dbus::MockTestData;

pub struct MockNetworkManager {
    pub network_manager_base: dbus_crossroads::IfaceToken<MockTestData>,
    pub network_manager_settings: dbus_crossroads::IfaceToken<MockTestData>,
    pub network_manager_device: dbus_crossroads::IfaceToken<MockDeviceData>,
    pub network_manager_access_point: dbus_crossroads::IfaceToken<MockAccessPointData>,
    pub network_manager_active_connection: dbus_crossroads::IfaceToken<MockTestData>,
    pub network_manager_data: MockNetworkData,
}

pub fn mock_network_manager(
    cross: &mut Crossroads,
    conn: Arc<SyncConnection>,
) -> MockNetworkManager {
    let mut interfaces = MockNetworkManager {
        network_manager_base: mock_network_manager_base(cross),
        network_manager_settings: mock_network_manager_settings(cross),
        network_manager_device: mock_network_manager_device(cross, conn),
        network_manager_access_point: mock_network_manager_access_points(cross),
        network_manager_active_connection: mock_network_manager_active_connection(cross),
        network_manager_data: MockNetworkData::new(),
    };

    create_mock_devices(cross, &mut interfaces, 3);
    create_mock_access_points(cross, &mut interfaces, 3);

    interfaces
}

pub fn mock_network_manager_base(
    cross: &mut Crossroads,
) -> dbus_crossroads::IfaceToken<MockTestData> {
    cross.register(NM_INTERFACE!(), |c| {
        c.property("ActiveConnections")
            .get(|_, cross: &mut MockTestData| {
                Ok(cross
                    .network_data
                    .network_manager_data
                    .active_connections
                    .clone())
            });
        c.method_with_cr_async("Test", (), (), move |mut ctx, _, ()| async move {
            ctx.reply(Ok(()))
        });
        c.method_with_cr_async(
            "GetAllDevices",
            (),
            ("devices",),
            move |mut ctx, cross, ()| {
                let data: &mut MockTestData = cross.data_mut(ctx.path()).unwrap();
                let devices = data.network_data.network_manager_data.devices.clone();
                async move { ctx.reply(Ok((devices,))) }
            },
        );
        c.method_with_cr_async(
            "AddAndActivateConnection",
            ("connection", "device", "specific_object"),
            ("path", "active_connection"),
            move |mut ctx,
                  _,
                  (_connection, _device, _specific_object): (
                PropMap,
                Path<'static>,
                Path<'static>,
            )| async move {
                // noop
                let path = Path::from("/");
                let active_connection = Path::from("/");
                ctx.reply(Ok((path, active_connection)))
            },
        );
    })
}

pub fn mock_network_manager_settings(
    cross: &mut Crossroads,
) -> dbus_crossroads::IfaceToken<MockTestData> {
    cross.register(NM_SETTINGS_INTERFACE!(), |c| {
        c.method_with_cr_async("Test", (), (), move |mut ctx, _, ()| async move {
            ctx.reply(Ok(()))
        });
        c.method_with_cr_async(
            "ListConnections",
            (),
            ("connections",),
            move |mut ctx, cross, ()| {
                let data: &mut MockTestData = cross.data_mut(ctx.path()).unwrap();
                let connections = data.network_data.network_manager_data.connections.clone();
                async move { ctx.reply(Ok((connections,))) }
            },
        );
    })
}

pub fn mock_network_manager_device(
    cross: &mut Crossroads,
    conn: Arc<SyncConnection>,
) -> dbus_crossroads::IfaceToken<MockDeviceData> {
    let conn_added = conn.clone();
    let conn_removed = conn.clone();
    cross.register(NM_DEVICE_INTERFACE!(), |c| {
        c.signal::<(Path<'static>,), _>("AccessPointAdded", ("access_point",));
        c.signal::<(Path<'static>,), _>("AccessPointRemoved", ("access_point",));
        c.property("DeviceType")
            .get(|_, cross: &mut MockDeviceData| Ok(cross.device_type));
        c.property("Interface")
            .get(|_, _: &mut MockDeviceData| Ok("Mock".to_string()));
        c.method_with_cr_async(
            "GetAllAccessPoints",
            (),
            ("access_points",),
            move |mut ctx, cross, ()| {
                let data: &mut MockDeviceData = cross.data_mut(ctx.path()).unwrap();
                let connections = data.access_points.clone();
                async move { ctx.reply(Ok((connections,))) }
            },
        );
        c.method_with_cr_async(
            "CreateFakeAddedSignal",
            (),
            (),
            move |mut ctx, cross, ()| {
                let new_path = "/org/Xebito/ReSet/Test/AccessPoint/100";
                let interface: dbus_crossroads::IfaceToken<MockAccessPointData>;
                {
                    let data: &mut MockDeviceData = cross.data_mut(ctx.path()).unwrap();
                    interface = data.access_point_interface;
                }
                cross.insert(new_path, &[interface], MockAccessPointData::new(100));
                let data: &mut MockDeviceData = cross.data_mut(ctx.path()).unwrap();
                data.access_points
                    .push(new_path.into());
                let msg = Message::signal(
                    &ctx.path().clone(),
                    &NM_DEVICE_INTERFACE!().into(),
                    &"AccessPointAdded".into(),
                )
                .append1(Path::from(new_path));
                conn_added.send(msg).expect("Could not send signal");
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "CreateFakeRemovedSignal",
            (),
            (),
            move |mut ctx, cross, ()| {
                let new_path = "/org/Xebito/ReSet/Test/AccessPoint/100";
                cross.remove::<MockDeviceData>(&Path::from(new_path));
                let msg = Message::signal(
                    &ctx.path().clone(),
                    &NM_DEVICE_INTERFACE!().into(),
                    &"AccessPointRemoved".into(),
                )
                .append1(Path::from(new_path));
                conn_removed.send(msg).expect("Could not send signal");
                let data: &mut MockDeviceData = cross.data_mut(ctx.path()).unwrap();
                data.access_points.remove(0);
                async move { ctx.reply(Ok(())) }
            },
        );
    })
}

pub fn mock_network_manager_access_points(
    cross: &mut Crossroads,
) -> dbus_crossroads::IfaceToken<MockAccessPointData> {
    cross.register(NM_ACCESS_POINT_INTERFACE!(), |c| {
        c.property("Ssid")
            .get(|_, cross: &mut MockAccessPointData| Ok(cross.ssid.clone()));
        c.property("Strength")
            .get(|_, cross: &mut MockAccessPointData| Ok(cross.strength));
        c.method_with_cr_async(
            "AddAndActivateConnection",
            ("connection",),
            ("path",),
            move |mut ctx, _, (_connection,): (Path<'static>,)| async move {
                // noop
                let path = Path::from("/");
                ctx.reply(Ok((path,)))
            },
        );
    })
}

pub fn mock_network_manager_active_connection(
    cross: &mut Crossroads,
) -> dbus_crossroads::IfaceToken<MockTestData> {
    cross.register(NM_ACTIVE_CONNECTION_INTERFACE!(), |c| {
        c.method_with_cr_async(
            "AddAndActivateConnection",
            ("connection",),
            ("path",),
            move |mut ctx, _, (_connection,): (Path<'static>,)| async move {
                // noop
                let path = Path::from("/");
                ctx.reply(Ok((path,)))
            },
        );
    })
}

pub struct MockNetworkData {
    access_points: Vec<Path<'static>>,
    devices: Vec<Path<'static>>,
    current_device: Device,
    connections: Vec<Path<'static>>,
    active_connections: Vec<Path<'static>>,
}

impl MockNetworkData {
    pub fn new() -> Self {
        // TODO: add data for tests
        MockNetworkData {
            access_points: Vec::new(),
            devices: Vec::new(),
            current_device: Device::new(Path::from("/"), "none".to_string()),
            connections: vec![
                Path::from(NM_DEVICES_PATH!().to_string() + "/Connection1"),
                Path::from(NM_DEVICES_PATH!().to_string() + "/Connection2"),
                Path::from(NM_DEVICES_PATH!().to_string() + "/Connection3"),
            ],
            active_connections: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct MockAccessPointData {
    ssid: Vec<u8>,
    strength: u8,
}

impl MockAccessPointData {
    fn new(id: u32) -> Self {
        Self {
            ssid: ("accesspoint".to_string() + &id.to_string()).into(),
            strength: 150,
        }
    }
}

#[derive(Clone)]
pub struct MockDeviceData {
    device_type: u32,
    access_points: Vec<Path<'static>>,
    access_point_interface: dbus_crossroads::IfaceToken<MockAccessPointData>,
}

impl MockDeviceData {
    fn new(id: u32, access_point_interface: dbus_crossroads::IfaceToken<MockAccessPointData>) -> Self {
        Self {
            device_type: 2,
            access_points: vec![Path::from(
                "/org/Xetibo/ReSet/Test/AccessPoint/".to_string() + &id.to_string(),
            )],
            access_point_interface
        }
    }
}

pub fn create_mock_devices(
    cross: &mut Crossroads,
    network_interfaces: &mut MockNetworkManager,
    amount: u32,
) {
    for i in 0..amount {
        let path = "/org/Xetibo/ReSet/Test/Devices/".to_string() + &i.to_string();
        cross.insert(
            path.clone(),
            &[network_interfaces.network_manager_device],
            MockDeviceData::new(i, network_interfaces.network_manager_access_point),
        );
        network_interfaces
            .network_manager_data
            .devices
            .push(Path::from(path));
    }
}

pub fn create_mock_access_points(
    cross: &mut Crossroads,
    network_interfaces: &mut MockNetworkManager,
    amount: u32,
) {
    for i in 0..amount {
        let path = "/org/Xetibo/ReSet/Test/AccessPoint/".to_string() + &i.to_string();
        cross.insert(
            path.clone(),
            &[network_interfaces.network_manager_access_point],
            MockAccessPointData::new(i),
        );
        network_interfaces
            .network_manager_data
            .access_points
            .push(Path::from(path));
    }
}
