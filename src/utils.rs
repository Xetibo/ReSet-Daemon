use std::collections::HashMap;

use dbus::{
    arg::{RefArg, Variant},
    Path,
};
use ReSet_Lib::audio::audio::{Card, InputStream, OutputStream, Sink, Source};

pub type MaskedPropMap = HashMap<String, HashMap<String, Variant<Box<dyn RefArg>>>>;

pub type FullMaskedPropMap = HashMap<
    Path<'static>,
    HashMap<std::string::String, HashMap<std::string::String, dbus::arg::Variant<Box<dyn RefArg>>>>,
>;

pub enum AudioRequest {
    ListSources,
    GetDefaultSource,
    SetSourceVolume(u32, u16, u32),
    SetSourceMute(u32, bool),
    SetDefaultSource(String),
    ListSinks,
    GetDefaultSink,
    SetSinkVolume(u32, u16, u32),
    SetSinkMute(u32, bool),
    SetDefaultSink(String),
    ListInputStreams,
    SetSinkOfInputStream(u32, u32),
    SetInputStreamVolume(u32, u16, u32),
    SetInputStreamMute(u32, bool),
    ListOutputStreams,
    SetSourceOfOutputStream(u32, u32),
    SetOutputStreamVolume(u32, u16, u32),
    SetOutputStreamMute(u32, bool),
    ListCards,
    SetCardProfileOfDevice(u32, String),
    StopListener,
}

pub enum AudioResponse {
    DefaultSink(Sink),
    DefaultSource(Source),
    Sources(Vec<Source>),
    Sinks(Vec<Sink>),
    InputStreams(Vec<InputStream>),
    OutputStreams(Vec<OutputStream>),
    Cards(Vec<Card>),
}
