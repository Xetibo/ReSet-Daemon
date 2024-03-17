use re_set_lib::utils::plugin::{Plugin, PluginCapabilities};

extern "C" {
    /// The startup function is intended to be used to allocate any required resources.
    pub fn startup();

    /// Cleanup any resources allocated for your plugin that aren't automatically removed.
    pub fn shutdown();

    /// Reports the capabilities that your plugin will provide, simply return a vector of strings.
    #[allow(improper_ctypes)]
    pub fn capabilities() -> PluginCapabilities;

    /// Inserts your plugin interface into the dbus server.
    #[allow(improper_ctypes)]
    pub fn dbus_interface() -> Plugin;

    /// Use this function to return any tests you would like to have run.
    /// This might be a bit confusing as this will force you to define your functions for testing
    /// outside of your typical rust tests.
    pub fn run_tests();
}
