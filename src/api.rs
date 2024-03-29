// TODO add event API
/// # ReSet-Daemon API
/// This is the DBus API documentation for the ReSet-Daemon.
/// Please note that the functions are shown in rust format, the actual functions are regular dbus
/// functions and have to be used as such.
///
/// ## DBus Types
/// y: u8\
/// b: bool\
/// n: i32\
/// q: u16\
/// i: i32\
/// u: u32\
/// x: i64\
/// t: u64\
/// d: f64\
/// o: `Path<'static>` this is the object path\
/// a: `Vec<T>` an array of something
#[allow(non_snake_case)]
pub mod API {
    use dbus::{arg::PropMap, Path};
    use re_set_lib::{
        audio::audio_structures::{Card, InputStream, OutputStream, Sink, Source},
        bluetooth::bluetooth_structures::{BluetoothAdapter, BluetoothDevice},
        network::network_structures::{AccessPoint, WifiDevice},
    };
    use std::collections::HashMap;

    /// # Base API
    /// Simple API for connectivety checks and functionality check.
    ///
    /// DBus interface name: org.Xetibo.ReSet.Daemon
    ///
    #[allow(dead_code, non_snake_case)]
    pub trait BaseAPI {
        ///
        /// Returns all capabilities of the daemon as strings
        fn GetCapabilities() -> Vec<String>;
        ///
        /// Register the client to the daemon.\
        /// This is mainly useful for clients that want to ensure the daemon is running before
        /// starting calls.\
        /// Later on this can be expanded for more functionality.
        fn RegisterClient(client_name: String) -> bool;
        ///
        /// Deletes the entry for this client from the daemon.
        fn UnregisterClient(client_name: String) -> bool;
        ///
        /// Shuts down the daemon.
        fn Shutdown();
    }

    /// # Wireless Manager API
    /// The wireless manager handles connecting, disconnecting, configuring, saving and removing of wireless network
    /// connections.
    ///
    /// DBus interface name: org.Xetibo.ReSet.Wireless
    ///
    /// ## Types
    ///
    /// ### AccessPoint
    /// The AccessPoint has the following DBus signature: ayyoob\
    /// `Vec<u8>, u8, Path<'static>, Path<'static>, bool`
    ///
    /// ### WifiDevice
    /// The WifiDevice has the following DBus signature: osay\
    /// `Path<'static>,String, Vec<u8>`
    ///
    /// ## Events
    /// Removed events are done with paths since the actual data behind the specific object is
    /// already removed.
    ///
    /// AccessPointChanged -> AccessPoint\
    /// AccessPointAdded -> AccessPoint\
    /// AccessPointRemoved -> Path<'static>\
    /// WifiDeviceChanged -> WifiDevice\
    /// called after disabling wifi in order to provide functionality.\
    /// ResetWifiDevices -> Vec<WifiDevices>\
    ///
    pub trait WirelessAPI {
        ///
        /// Returns all access points for the current wireless network device.
        fn ListAccessPoints() -> Vec<AccessPoint>;
        ///
        /// A check that returns the current status of Wifi.\
        /// Returns a bool as a result of the operation.
        fn GetWifiStatus() -> bool;
        ///
        /// Enables or disables Wifi for the entire system.
        fn SetWifiEnabled(enabled: bool) -> bool;
        ///
        /// Returns the dbus path of the current wireless network device, as well as the name.
        fn GetCurrentWifiDevice() -> WifiDevice;
        ///
        /// Returns all available wireless network devices.
        fn GetAllWifiDevices() -> Vec<WifiDevice>;
        ///
        /// Sets the current network device based on the dbus path of the device.\
        /// Returns true on success and false on error.
        fn SetWifiDevice(device: Path<'static>) -> bool;
        ///
        /// Connects to an access point that has a known connection inside the NetworkManager.\
        /// Note, for a new access point, use the ConnectToNewAccessPoint function.\
        /// Returns true on success and false on error.
        fn ConnectToKnownAccessPoint(access_point: AccessPoint) -> bool;
        ///
        /// Connects to a new access point with a password.\
        /// Returns true on success and false on error.
        fn ConnectToNewKnownAccessPoint(access_point: AccessPoint, password: String) -> bool;
        ///
        /// Disconnects from the currently connected access point.\
        /// Calling this without a connected access point will return false.\
        /// Returns true on success and false on error.
        fn DisconnectFromCurrentAccessPoint() -> bool;
        ///
        /// Returns the stored connections for the currently selected wireless device from NetworkManager.\
        /// Returns dbus invalid arguments on error.
        fn ListStoredConnections() -> Vec<(Path<'static>, Vec<u8>)>;
        ///
        /// Returns the settings of a connection.\
        /// Can be used in combination with the Connection struct in order to provide easy serialization
        /// and deserialization from and to this hashmap.\
        /// Returns dbus invalid arguments on error.
        fn GetConnectionSettings(path: Path<'static>) -> HashMap<String, PropMap>;
        ///
        /// Sets the settings of a connection.\
        /// Can be used in combination with the Connection struct in order to provide easy serialization
        /// and deserialization from and to this hashmap.\
        /// Returns true on success and false on error.
        fn SetConnectionSettings(path: Path<'static>, settings: HashMap<String, PropMap>) -> bool;
        ///
        /// Deletes the stored connection given the dbus path.\
        /// Returns true on success and false on error.
        fn DeleteConnection(path: Path<'static>) -> bool;
        ///
        /// Starts the wireless network listener which provides dbus events on access points and the
        /// wireless device.\
        /// Repeatedly starting the network listener twice will simply return an error on consecutive
        /// tries.\
        /// Returns true on success and false on error.
        fn StartNetworkListener() -> bool;
        ///
        /// Stops the wireless network listener.\
        /// Returns true on success and false on error.
        fn StopNetworkListener() -> bool;
    }

    /// # Bluetooth Manager API
    /// Handles connecting and disconnecting Bluetooth devices.
    ///
    /// DBus interface name: org.Xetibo.ReSet.Bluetooth
    ///
    /// ## Types
    ///
    /// ### BluetoothDevice
    /// The BluetoothDevice has the following DBus signature: onssobbbbbss\
    /// `Path<'static>, u16, String, String, Path<'static>, bool, bool, bool, bool, bool, String, String`
    ///
    /// ### BluetoothAdapter
    /// The BluetoothAdapter has the following DBus signature: osbbb\
    /// `Path<'static>, String, bool, bool, bool`
    ///
    /// ## Events
    /// Removed events are done with paths since the actual data behind the specific object is
    /// already removed.
    ///
    /// BluetoothDeviceAdded -> BluetoothDevice\
    /// BluetoothDeviceRemoved -> Path<'static>\
    /// BluetoothDeviceChanged -> BluetoothDevice
    ///
    /// ## Agent Events
    /// NOTE Currently unused
    ///
    /// PincodeRequested -> ()\
    /// DisplayPinCode -> ()\
    /// PassKeyRequested -> ()\
    /// DisplayPassKey -> (u32, u16)\
    /// PinCodeRequested -> ()
    ///
    pub trait BluetoothAPI {
        ///
        /// Starts searching for Bluetooth devices.\
        /// Note this is without a listener, you would have to manually request Bluetooth devices.
        fn StartBluetoothSearch();
        ///
        /// Stops searching for Bluetooth devices.
        fn StopBluetoothSearch();
        ///
        /// Starts the listener for Bluetooth events for a specified duration.\
        /// Repeatedly starting the network listener while already active will do nothing.
        fn StartBluetoothListener();
        ///
        /// Stops the listener for Bluetooth events.\
        fn StopBluetoothListener();
        ///
        /// Returns the currently available Bluetooth adapters.
        fn GetBluetoothAdapters() -> Vec<BluetoothAdapter>;
        ///
        /// Returns the current default Bluetooth adapter.
        fn GetCurrentBluetoothAdapter() -> BluetoothAdapter;
        ///
        /// Sets the default Bluetooth adapter.\
        /// The path can be found inside the BluetoothAdapter struct.
        fn SetBluetoothAdapter(path: Path<'static>) -> bool;
        ///
        /// Sets the discoverability of a specific Bluetooth adapter.
        fn SetBluetoothAdapterDiscoverability(path: Path<'static>, enabled: bool) -> bool;
        ///
        /// Sets the pairability of a specific Bluetooth adapter.
        fn SetBluetoothAdapterPairability(path: Path<'static>, enabled: bool) -> bool;
        ///
        /// Returns the currently available Bluetooth devices.
        /// Only returns devices provided by bluetooth discovery.
        /// Use StartBluetoothScan in order to enable temporary discovery.
        fn GetBluetoothDevices() -> Vec<BluetoothDevice>;
        ///
        /// Connects to a Bluetooth device given the DBus path.\
        /// Note that this requires an existing pairing.\
        /// Returns true on success and false on error.
        fn ConnectToBluetoothDevice(path: Path<'static>) -> bool;
        ///
        /// Pairs with a Bluetooth device given the DBus path.\
        /// Initiates the pairing process which is handled by the Bluetooth Agent.\
        /// Returns true on success and false on error.
        /// NOTE: THIS IS CURRENTLY DISABLED!
        fn PairWithBluetoothDevice(path: Path<'static>) -> bool;
        ///
        /// Disconnects a Bluetooth device given the DBus path.
        /// Returns true on success and false on error.
        fn DisconnectFromBluetoothDevice(path: Path<'static>) -> bool;
        ///
        /// This will remove the pairing on the Bluetooth device.
        fn RemoveDevicePairing(path: Path<'static>) -> bool;
        ///
        /// Returns all connected Bluetooth devices.
        /// The first part of the HashMap is the DBus path of the object, the second is the object
        /// itself.
        fn GetConnectedBluetoothDevices() -> Vec<BluetoothDevice>;
    }

    /// # Audio Manager API
    /// Handles volume of both devices and streams, as well as default devices for each stream, and the
    /// default devices in general.\
    /// In addition, each device can be configured with a profile and each device can be turned off via
    /// Pulse cards.
    ///
    /// ## Interface
    /// DBus interface name: org.Xetibo.ReSet.Audio
    ///
    /// ## Types
    ///
    /// ### Source
    /// The Source has the following DBus signature: ussqaubi\
    /// `u32, String, String, u16, Vec<u32>, bool, i32`
    ///
    /// ### Sink
    /// The Sink has the following DBus signature: ussqaubi\
    /// `u32, String, String, u16, Vec<u32>, bool, i32`
    ///
    /// ### InputStream
    /// The InputStream has the following DBus signature: ussuqaubb\
    /// `u32, String, String, u32, u16, Vec<u32>, bool, bool`
    ///
    /// ### OutputStream
    /// The OutputStream has the following DBus signature: ussuqaubb\
    /// `u32, String, String, u32, u16, Vec<u32>, bool, bool`
    ///
    /// ### Card
    /// The Card has the following DBus signature: a(ussuqaubb)\
    /// `Vec<(u32, String, String, u32, u16, Vec<u32>, bool, bool)>`
    ///
    /// ## Events
    /// Removed events are done with paths since the actual data behind the specific object is
    /// already removed.
    ///
    /// SinkChanged -> Sink\
    /// SinkAdded -> Sink\
    /// SinkRemoved -> Path<'static>\
    /// SourceChanged -> Source\
    /// SourceAdded -> Source\
    /// SourceRemoved -> Path<'static>\
    /// InputStreamChanged -> InputStream\
    /// InputStreamAdded -> InputStream\
    /// InputStreamRemoved -> Path<'static>\
    /// OutputStreamChanged -> OutputStream\
    /// OutputStreamAdded -> OutputStream\
    /// OutputStreamRemoved -> Path<'static>
    ///
    pub trait AudioAPI {
        ///
        /// Returns the default sink(speaker, headphones, etc.) from pulseaudio.\
        fn GetDefaultSink() -> Sink;
        ///
        /// Returns the default sink name(speaker, headphones, etc.) from pulseaudio.\
        /// This is mainly useful for checking if an event-given sink is the default one, as this
        /// information is not within the sink struct for performance reasons.
        fn GetDefaultSinkName() -> String;
        ///
        /// Returns the default source(microphone) from pulseaudio.\
        fn GetDefaultSource() -> Source;
        ///
        /// Returns the default source name(microphone) from pulseaudio.\
        /// This is mainly useful for checking if an event-given source is the default one, as this
        /// information is not within the source struct for performance reasons.
        fn GetDefaultSourceName() -> String;
        ///
        /// Sets the default sink via name.(this is a pulse audio definition!)\
        /// The name can be found inside the Sink struct after calling ListSinks() or by listening to
        /// events.
        fn SetDefaultSink(sink: String) -> Sink;
        ///
        /// Sets the default sink via name.(this is a pulse audio definition!)\
        /// The name can be found inside the Sink struct after calling ListSinks() or by listening to
        /// events.
        fn SetDefaultSource(source: String) -> Source;
        ///
        /// Returns all current sinks.
        fn ListSinks() -> Vec<Sink>;
        ///
        /// Returns all current sources.
        fn ListSources() -> Vec<Source>;
        ///
        /// Returns all streams that are responsible for playing audio, e.g. applications.\
        fn ListInputStreams() -> Vec<InputStream>;
        ///
        /// Returns all streams that are responsible for recording audio, e.g. OBS, voice chat applications.\
        fn ListOutputStreams() -> Vec<OutputStream>;
        ///
        /// Returns the PulseAudio cards for every device. (The card holds information about all possible
        /// audio profiles and whether or not the device is disabled.)\
        fn ListCards() -> Vec<Card>;
        ///
        /// Sets the default volume of the sink on all channels to the specified value.\
        /// Currently ReSet does not offer individual channel volumes. (This will be added later)\
        /// The index can be found within the Sink data structure.
        fn SetSinkVolume(index: u32, channels: u16, volume: u32);
        ///
        /// Sets the mute state of the sink.\
        /// True -> muted, False -> unmuted\
        /// The index can be found within the Sink data structure.
        fn SetSinkMute(index: u32, muted: bool);
        ///
        /// Sets the default volume of the source on all channels to the specified value.\
        /// Currently ReSet does not offer individual channel volumes. (This will be added later)\
        /// The index can be found within the Source data structure.
        fn SetSourceVolume(index: u32, channels: u16, volume: u32);
        ///
        /// Sets the mute state of the source.\
        /// True -> muted, False -> unmuted\
        /// The index can be found within the Source data structure.
        fn SetSourceMute(index: u32, muted: bool);
        ///
        /// Sets the default volume of the input_stream on all channels to the specified value.\
        /// Currently ReSet does not offer individual channel volumes. (This will be added later)\
        /// The index can be found within the InputStream data structure.
        fn SetSinkOfInputStream(input_stream: u32, sink: u32);
        ///
        /// Sets the default volume of the input stream on all channels to the specified value.\
        /// Currently ReSet does not offer individual channel volumes. (This will be added later)\
        /// The index can be found within the InputStream data structure.
        fn SetInputStreamVolume(index: u32, channels: u16, volume: u32);
        ///
        /// Sets the mute state of the input stream.\
        /// True -> muted, False -> unmuted\
        /// The index can be found within the InputStream data structure.
        fn SetInputStreamMute(index: u32, muted: bool);
        ///
        /// Sets the target source of an output stream. (The target input device for an application)\
        /// Both the output stream and the source are indexes, they can be found within their respective
        /// data structure.
        fn SetSourceOfOutputStream(output_stream: u32, source: u32);
        ///
        /// Sets the default volume of the output stream on all channels to the specified value.\
        /// Currently ReSet does not offer individual channel volumes. (This will be added later)\
        /// The index can be found within the OutputStream data structure.
        fn SetOutputStreamVolume(index: u32, channels: u16, volume: u32);
        ///
        /// Sets the mute state of the output stream.\
        /// True -> muted, False -> unmuted\
        /// The index can be found within the OutputStream data structure.
        fn SetOutputStreamMute(index: u32, muted: bool);
        ///
        /// Sets the profile for a device according to the name of the profile.\
        /// The available profile names can be found in the card of the device, which can be received with
        /// the ListCards() function.\
        /// The index of the device can be found in the Device data structure.
        fn SetCardOfDevice(device_index: u32, profile_name: String);
    }
}
