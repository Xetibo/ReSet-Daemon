use std::collections::HashMap;

use dbus::{
    arg::{RefArg, Variant},
    Path,
};

pub type MaskedPropMap = HashMap<String, HashMap<String, Variant<Box<dyn RefArg>>>>;

pub type FullMaskedPropMap = HashMap<
    Path<'static>,
    HashMap<std::string::String, HashMap<std::string::String, dbus::arg::Variant<Box<dyn RefArg>>>>,
>;
