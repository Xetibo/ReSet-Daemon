use dbus as dbus;
#[allow(unused_imports)]
use dbus::arg;
use dbus::blocking;

pub trait OrgFreedesktopDBusIntrospectable {
    fn introspect(&self) -> Result<String, dbus::Error>;
}

impl<'a, T: blocking::BlockingSender, C: ::std::ops::Deref<Target=T>> OrgFreedesktopDBusIntrospectable for blocking::Proxy<'a, C> {

    fn introspect(&self) -> Result<String, dbus::Error> {
        self.method_call("org.freedesktop.DBus.Introspectable", "Introspect", ())
            .and_then(|r: (String, )| Ok(r.0, ))
    }
}

pub trait OrgFreedesktopDBusObjectManager {
    fn get_managed_objects(&self) -> Result<::std::collections::HashMap<dbus::Path<'static>, ::std::collections::HashMap<String, arg::PropMap>>, dbus::Error>;
}

#[derive(Debug)]
pub struct InterfacesAddedSignal {
    pub object: dbus::Path<'static>,
    pub interfaces: ::std::collections::HashMap<String, arg::PropMap>,
}

impl arg::AppendAll for InterfacesAddedSignal {
    fn append(&self, i: &mut arg::IterAppend) {
        arg::RefArg::append(&self.object, i);
        arg::RefArg::append(&self.interfaces, i);
    }
}

impl arg::ReadAll for InterfacesAddedSignal {
    fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
        Ok(InterfacesAddedSignal {
            object: i.read()?,
            interfaces: i.read()?,
        })
    }
}

impl dbus::message::SignalArgs for InterfacesAddedSignal {
    const NAME: &'static str = "InterfacesAdded";
    const INTERFACE: &'static str = "org.freedesktop.DBus.ObjectManager";
}

#[derive(Debug)]
pub struct OrgFreedesktopDBusObjectManagerInterfacesRemoved {
    pub object: dbus::Path<'static>,
    pub interfaces: Vec<String>,
}

impl arg::AppendAll for OrgFreedesktopDBusObjectManagerInterfacesRemoved {
    fn append(&self, i: &mut arg::IterAppend) {
        arg::RefArg::append(&self.object, i);
        arg::RefArg::append(&self.interfaces, i);
    }
}

impl arg::ReadAll for OrgFreedesktopDBusObjectManagerInterfacesRemoved {
    fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
        Ok(OrgFreedesktopDBusObjectManagerInterfacesRemoved {
            object: i.read()?,
            interfaces: i.read()?,
        })
    }
}

impl dbus::message::SignalArgs for OrgFreedesktopDBusObjectManagerInterfacesRemoved {
    const NAME: &'static str = "InterfacesRemoved";
    const INTERFACE: &'static str = "org.freedesktop.DBus.ObjectManager";
}

impl<'a, T: blocking::BlockingSender, C: ::std::ops::Deref<Target=T>> OrgFreedesktopDBusObjectManager for blocking::Proxy<'a, C> {

    fn get_managed_objects(&self) -> Result<::std::collections::HashMap<dbus::Path<'static>, ::std::collections::HashMap<String, arg::PropMap>>, dbus::Error> {
        self.method_call("org.freedesktop.DBus.ObjectManager", "GetManagedObjects", ())
            .and_then(|r: (::std::collections::HashMap<dbus::Path<'static>, ::std::collections::HashMap<String, arg::PropMap>>, )| Ok(r.0, ))
    }
}
