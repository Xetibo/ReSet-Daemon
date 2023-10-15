use dbus::{blocking::Connection, Path};
use dbus_crossroads::Crossroads;

use super::{
    bluetooth::BluetoothInterface,
    network::{get_wifi_devices, AccessPoint, Device, Error},
};

#[derive(Clone)]
pub struct DaemonData {
    pub n_devices: Vec<Device>,
    pub current_n_device: Device,
    pub b_interface: BluetoothInterface,
}

pub struct Daemon {
    pub n_connection: Connection,
    pub data: DaemonData,
}

impl Daemon {
    pub fn create() -> Result<Self, Error> {
        let mut n_devices = get_wifi_devices();
        if n_devices.len() < 1 {
            return Err(Error {
                message: "Could not get any wifi devices",
            });
        }
        let current_n_device = n_devices.pop().unwrap();
        let b_interface_opt = BluetoothInterface::create();
        let b_interface: BluetoothInterface;
        if b_interface_opt.is_none() {
            b_interface = BluetoothInterface::empty();
        } else {
            b_interface = b_interface_opt.unwrap();
        }
        Ok(Self {
            n_connection: Connection::new_session().unwrap(),
            data: DaemonData {
                n_devices,
                current_n_device,
                b_interface,
            },
        })
    }

    pub fn run(&mut self) {
        self.n_connection
            .request_name("org.xetibo.ReSet", false, true, false)
            .unwrap();
        let mut cross = Crossroads::new();
        let token = cross.register("org.xetibo.ReSet", |c| {
            c.method(
                "ListAccessPoints",
                (),
                ("access_points",),
                move |_, d: &mut DaemonData, ()| {
                    let access_points = d.current_n_device.get_access_points();
                    dbg!(access_points.clone());
                    Ok((d.current_n_device.get_access_points(),))
                },
            );
            c.method(
                "ConnectToKnownAccessPoint",
                ("access_point",),
                ("result",),
                move |_, d: &mut DaemonData, (access_point,): (AccessPoint,)| {
                    let res = d.current_n_device.connect_to_access_point(access_point);
                    if res.is_err() {
                        return Ok((false,));
                    }
                    Ok((true,))
                },
            );
            c.method(
                "ConnectToNewAccessPoint",
                ("access_point", "password"),
                ("result",),
                move |_, d: &mut DaemonData, (access_point, password): (AccessPoint, String)| {
                    let res = d
                        .current_n_device
                        .add_and_connect_to_access_point(access_point, password);
                    if res.is_err() {
                        return Ok((false,));
                    }
                    Ok((true,))
                },
            );
            c.method(
                "DisconnectFromCurrentAccessPoint",
                (),
                ("result",),
                move |_, d: &mut DaemonData, ()| {
                    let res = d.current_n_device.disconnect_from_current();
                    if res.is_err() {
                        return Ok((false,));
                    }
                    Ok((true,))
                },
            );
            c.method(
                "StartBluetoothSearch",
                (),
                ("result",),
                move |_, d: &mut DaemonData, ()| {
                    let res = d.b_interface.start_discovery();
                    if res.is_err() {
                        return Ok((false,));
                    }
                    Ok((true,))
                },
            );
            c.method(
                "StopBluetoothSearch",
                (),
                ("result",),
                move |_, d: &mut DaemonData, ()| {
                    let res = d.b_interface.stop_discovery();
                    if res.is_err() {
                        return Ok((false,));
                    }
                    Ok((true,))
                },
            );
        });
        cross.insert("/org/xetibo/ReSet/Network", &[token], self.data.clone());
        cross.insert("/org/xetibo/ReSet/Bluetooth", &[token], self.data.clone());
        cross.serve(&self.n_connection).unwrap();
    }
}
