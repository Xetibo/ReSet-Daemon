use crate::network::network_manager::Device;
use dbus::{arg::PropMap, Path};
use dbus_crossroads::Crossroads;
use re_set_lib::network::network_structures::AccessPoint;

use super::mock_dbus::MockTestData;

pub fn mock_network_interface(
    cross: &mut Crossroads,
) -> Vec<dbus_crossroads::IfaceToken<MockTestData>> {
    let mut tokens = Vec::new();
    tokens.push(cross.register(NM_INTERFACE!(), |c| {
        c.method_with_cr_async(
            "GetAllDevices",
            (),
            ("devices",),
            move |mut ctx, cross, ()| {
                let data: &mut MockTestData = cross.data_mut(ctx.path()).unwrap();
                let devices = data.network_data.devices.clone();
                async move { ctx.reply(Ok((devices,))) }
            },
        );
        c.method_with_cr_async(
            "AddAndActivateConnection",
            ("connection", "device", "specific_object"),
            ("path", "active_connection"),
            move |mut ctx,
                  _,
                  (connection, device, specific_object): (
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
    }));
    tokens.push(cross.register(NM_SETTINGS_INTERFACE!(), |c| {
        c.method_with_cr_async("Test", (), (), move |mut ctx, _, ()| async move {
            ctx.reply(Ok(()))
        });
        c.method_with_cr_async(
            "ListConnections",
            (),
            ("connections",),
            move |mut ctx, cross, ()| {
                let data: &mut MockTestData = cross.data_mut(ctx.path()).unwrap();
                let connections = data.network_data.connections.clone();
                async move { ctx.reply(Ok((connections,))) }
            },
        );
    }));
    tokens.push(cross.register(NM_DEVICE_INTERFACE!(), |c| {
        c.method_with_cr_async(
            "GetAllAccessPoints",
            (),
            ("access_points",),
            move |mut ctx, cross, ()| {
                let data: &mut MockTestData = cross.data_mut(ctx.path()).unwrap();
                let connections = data.network_data.access_points.clone();
                async move { ctx.reply(Ok((connections,))) }
            },
        );
    }));
    tokens.push(cross.register(NM_ACCESS_POINT_INTERFACE!(), |c| {
        c.method_with_cr_async(
            "AddAndActivateConnection",
            ("connection",),
            ("path",),
            move |mut ctx, _, (connection,): (Path<'static>,)| async move {
                // noop
                let path = Path::from("/");
                ctx.reply(Ok((path,)))
            },
        );
    }));
    tokens.push(cross.register(NM_ACTIVE_CONNECTION_INTERFACE!(), |c| {
        c.method_with_cr_async(
            "AddAndActivateConnection",
            ("connection",),
            ("path",),
            move |mut ctx, _, (connection,): (Path<'static>,)| async move {
                // noop
                let path = Path::from("/");
                ctx.reply(Ok((path,)))
            },
        );
    }));
    tokens
}

pub struct MockNetworkData {
    access_points: Vec<Path<'static>>,
    devices: Vec<Path<'static>>,
    current_device: Device,
    connections: Vec<Path<'static>>,
}

impl MockNetworkData {
    pub fn new() -> Self {
        // TODO: add data for tests
        MockNetworkData {
            access_points: vec![
                Path::from(NM_ACCESS_POINT_PATH!().to_string() + "/AccessPoint1"),
                Path::from(NM_ACCESS_POINT_PATH!().to_string() + "/AccessPoint2"),
                Path::from(NM_ACCESS_POINT_PATH!().to_string() + "/AccessPoint3"),
            ],
            devices: vec![
                Path::from(NM_ACCESS_POINT_PATH!().to_string() + "/Device1"),
                Path::from(NM_ACCESS_POINT_PATH!().to_string() + "/Device2"),
                Path::from(NM_ACCESS_POINT_PATH!().to_string() + "/Device3"),
            ],
            current_device: Device::new(Path::from("/"), "none".to_string()),
            connections: vec![
                Path::from(NM_DEVICES_PATH!().to_string() + "/Connection1"),
                Path::from(NM_DEVICES_PATH!().to_string() + "/Connection2"),
                Path::from(NM_DEVICES_PATH!().to_string() + "/Connection3"),
            ],
        }
    }
}
