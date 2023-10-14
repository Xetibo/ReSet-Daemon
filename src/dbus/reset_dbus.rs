use dbus::{blocking::Connection, Path};
use dbus_crossroads::Crossroads;

use super::network::{get_wifi_devices, AccessPoint, Device, Error};

pub struct Daemon {
    pub connection: Connection,
    pub devices: Vec<Device>,
    pub current_device: Device,
}

impl Daemon {
    pub fn create() -> Result<Self, Error> {
        let mut devices = get_wifi_devices();
        if devices.len() < 1 {
            return Err(Error {
                message: "Could not get any wifi devices",
            });
        }
        let current_device = devices.pop().unwrap();
        Ok(Self {
            connection: Connection::new_session().unwrap(),
            devices,
            current_device,
        })
    }

    pub fn run(&mut self) {
        self.connection
            .request_name("org.xetibo.ReSet", false, true, false)
            .unwrap();
        let mut cross = Crossroads::new();
        let token = cross.register("org.xetibo.ReSet", |c| {
            c.method(
                "ListAccessPoints",
                (),
                ("access_points",),
                move |_, device: &mut Device, ()| {
                    let access_points = device.get_access_points();
                    dbg!(access_points.clone());
                    Ok((device.get_access_points(),))
                },
            );
            c.method(
                "ConnectToKnownAccessPoint",
                ("access_point",),
                ("result",),
                move |_, device: &mut Device, (access_point,): (AccessPoint,)| {
                    let res = device.connect_to_access_point(access_point);
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
                move |_, device: &mut Device, (access_point, password): (AccessPoint, String)| {
                    let res = device.add_and_connect_to_access_point(access_point, password);
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
                move |_, device: &mut Device, ()| {
                    let res = device.disconnect_from_current();
                    if res.is_err() {
                        return Ok((false,));
                    }
                    Ok((true,))
                },
            );
        });
        cross.insert("/org/xetibo/ReSet", &[token], self.current_device.clone());
        cross.serve(&self.connection).unwrap();
    }
}
