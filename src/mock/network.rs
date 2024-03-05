use crate::network::network_manager::Device;
use dbus::{arg::PropMap, Path};
use dbus_crossroads::Crossroads;
use re_set_lib::network::network_structures::AccessPoint;

use super::mock_dbus::MockTestData;

const MOCK_NETWORKMANAGER: &str = "org.Xetibo.ReSet.Test.Network";
const MOCK_NETWORKSETTINGS: &str = "org.Xetibo.ReSet.Test.Settings";
const MOCK_DEVICES: &str = "org.Xetibo.ReSet.Test.Devices";
const MOCK_ACCESSPOINTS: &str = "org.Xetibo.ReSet.Test.AccessPoints";
const MOCK_ACTIVEACCESSPOINT: &str = "org.Xetibo.ReSet.Test.ActiveAccessPoint";

pub fn mock_network_interface(
    cross: &mut Crossroads,
) -> Vec<dbus_crossroads::IfaceToken<MockTestData>> {
    let mut tokens = Vec::new();
    tokens.push(cross.register(MOCK_NETWORKMANAGER, |c| {
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
    tokens.push(cross.register(MOCK_NETWORKSETTINGS, |c| {
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
    tokens.push(cross.register(MOCK_DEVICES, |c| {
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
    tokens.push(cross.register(MOCK_ACCESSPOINTS, |c| {
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
    tokens.push(cross.register(MOCK_ACTIVEACCESSPOINT, |c| {
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
    access_points: Vec<AccessPoint>,
    devices: Vec<Device>,
}

impl MockNetworkData {
    pub fn new() -> Self {
        // TODO: add data for tests
        MockNetworkData {
            access_points: Vec::new(),
            devices: Vec::new(),
        }
    }
}
