// #[cfg(test)]
// macro_rules! CONSTANTS {
//     () => {
//         pub const DBUS_PATH: &str = "/org/Xetibo/ReSet/Test";
//         pub const WIRELESS: &str = "org.Xetibo.ReSet.Test.Network";
//         pub const BLUETOOTH: &str = "org.Xetibo.ReSet.Test.Bluetooth";
//     };
// }
//
// #[cfg(debug_assertions)]
// macro_rules! CONSTANTS {
//     () => {
//         pub const DBUS_PATH: &str = "/org/Xetibo/ReSet/Test";
//         pub const WIRELESS: &str = "org.Xetibo.ReSet.Test.Network";
//         pub const BLUETOOTH: &str = "org.Xetibo.ReSet.Test.Bluetooth";
//     };
// }

macro_rules! get_constants {
    () => {
        CONSTANTS.get().unwrap()
    };
}
