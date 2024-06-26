use dbus_crossroads::Crossroads;
use re_set_lib::utils::plugin::PluginCapabilities;

extern "C" {
    /// The startup function is intended to be used to allocate any required resources.
    pub fn backend_startup();

    /// Cleanup any resources allocated for your plugin that aren't automatically removed.
    pub fn backend_shutdown();

    /// Reports the capabilities that your plugin will provide, simply return a vector of strings.
    #[allow(improper_ctypes)]
    pub fn capabilities() -> PluginCapabilities;

    /// Reports the name of the plugin, used for duplication detection and plugin tests.
    #[allow(improper_ctypes)]
    pub fn name() -> String;

    /// Inserts your plugin interface into the dbus server.
    /// Provided as a parameter is the crossroads context, which you can use in order to insert your
    /// interfaces and objects
    #[allow(improper_ctypes)]
    pub fn dbus_interface(cross: &mut Crossroads);

    /// Use this function to return any tests you would like to have run.
    /// This might be a bit confusing as this will force you to define your functions for testing
    /// outside of your typical rust tests.
    pub fn backend_tests();
}
