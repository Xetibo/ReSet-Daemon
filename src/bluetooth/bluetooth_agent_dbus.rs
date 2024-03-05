use dbus::{Message, Path};
use dbus_crossroads::Crossroads;

use crate::{utils::CONSTANTS, DaemonData};

#[allow(dead_code)]
pub fn setup_bluetooth_agent(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register("org.bluez.Agent1", |c| {
        c.method(
            "RequestPinCode",
            ("device",),
            ("result",),
            move |ctx, d: &mut DaemonData, (_device,): (Path<'static>,)| {
                println!("pincode requested!");
                if d.bluetooth_agent.in_progress {
                    return Ok(("No pairing in progress.",));
                }
                let msg = Message::signal(
                    &Path::from(get_constants!().dbus_path),
                    &get_constants!().bluetooth.into(),
                    &"PincodeRequested".into(),
                );
                ctx.push_msg(msg);
                Ok(("",))
                // TODO handle receive with a dynamic dbus function? does that even exist?
            },
        );
        c.method(
            "DisplayPinCode",
            ("device", "pincode"),
            (),
            move |ctx, _d: &mut DaemonData, (_device, pincode): (Path<'static>, String)| {
                println!("display pincode");
                let msg = Message::signal(
                    &Path::from(get_constants!().dbus_path),
                    &get_constants!().bluetooth.into(),
                    &"DisplayPinCode".into(),
                )
                .append1(pincode);
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method(
            "RequestPasskey",
            ("device",),
            ("passkey",),
            move |ctx, _d: &mut DaemonData, (_device,): (Path<'static>,)| {
                println!("request passkey");
                let msg = Message::signal(
                    &Path::from(get_constants!().dbus_path),
                    &get_constants!().bluetooth.into(),
                    &"RequestPassKey".into(),
                );
                ctx.push_msg(msg);
                #[allow(clippy::unnecessary_cast)]
                Ok((0 as u32,))
                // leave me alone clippy, I am dealing with C code
            },
        );
        c.method(
            "DisplayPasskey",
            ("device", "passkey", "entered"),
            (),
            move |ctx,
                  _d: &mut DaemonData,
                  (_device, passkey, entered): (Path<'static>, u32, u16)| {
                println!("display passkey");
                let msg = Message::signal(
                    &Path::from(get_constants!().dbus_path),
                    &get_constants!().bluetooth.into(),
                    &"DisplayPassKey".into(),
                )
                .append2(passkey, entered);
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method(
            "RequestConfirmation",
            ("device", "passkey"),
            (),
            move |ctx, _d: &mut DaemonData, (_device, passkey): (Path<'static>, u32)| {
                println!("request confirmation");
                let msg = Message::signal(
                    &Path::from(get_constants!().dbus_path),
                    &get_constants!().bluetooth.into(),
                    &"RequestConfirmation".into(),
                )
                .append1(passkey);
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method(
            "RequestAuthorization",
            ("device",),
            (),
            move |ctx, _d: &mut DaemonData, (_device,): (Path<'static>,)| {
                println!("request authorization");
                let msg = Message::signal(
                    &Path::from(get_constants!().dbus_path),
                    &get_constants!().bluetooth.into(),
                    &"RequestAuthorization".into(),
                );
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method(
            "AuthorizeService",
            ("device", "uuid"),
            (),
            move |ctx, _d: &mut DaemonData, (_device, uuid): (Path<'static>, String)| {
                println!("authorize service");
                let msg = Message::signal(
                    &Path::from(get_constants!().dbus_path),
                    &get_constants!().bluetooth.into(),
                    &"AuthorizeService".into(),
                )
                .append1(uuid);
                ctx.push_msg(msg);
                Ok(())
            },
        );
        c.method("Cancel", (), (), move |_, d: &mut DaemonData, ()| {
            println!("called cancel");
            d.bluetooth_agent.in_progress = false;
            Ok(())
        });
        c.method("Release", (), (), move |_, d: &mut DaemonData, ()| {
            println!("called release");
            d.bluetooth_agent.in_progress = false;
            Ok(())
        });
    });

    token
}
