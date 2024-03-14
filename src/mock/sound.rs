// use super::mock_dbus::MockNetworkData;
// use dbus_crossroads::Crossroads;
//
// const MOCK_SOUND: &'static str = "MOCKsound";
//
// pub fn mock_sound_interface(
//     cross: &mut Crossroads,
// ) -> dbus_crossroads::IfaceToken<MockNetworkData> {
//     let token = cross.register(MOCK_SOUND, |c| {
//         println!("pingpang sound");
//     });
//     token
// }
// does this even make sense ?
// sound has no dbus, so we just require pulse ?
