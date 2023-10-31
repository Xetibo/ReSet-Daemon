use std::{cell::RefCell, ops::Deref, rc::Rc};

use std::sync::mpsc::{Receiver, Sender};

use dbus::{
    arg::{self, Append, Arg, ArgType, Get},
    Signature,
};
use pulse::context::introspect::{SinkInputInfo, SourceOutputInfo};
use pulse::volume::{ChannelVolumes, Volume};
use pulse::{
    self,
    callbacks::ListResult,
    context::{
        introspect::{SinkInfo, SourceInfo},
        Context, FlagSet,
    },
    mainloop::threaded::Mainloop,
    proplist::Proplist,
};

use crate::reset_dbus::{Request, Response};

pub struct PulseServer {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
    sender: Sender<Response>,
    receiver: Receiver<Request>,
}

#[derive(Debug)]
pub struct PulseError(&'static str);

pub struct Source {
    index: u32,
    name: String,
    alias: String,
    channels: u16,
    volume: Vec<u32>,
    muted: bool,
}

impl Append for Source {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(&self.index);
            i.append(&self.name);
            i.append(&self.alias);
            i.append(&self.channels);
            i.append(&self.volume);
            i.append(&self.muted);
        });
    }
}

impl<'a> Get<'a> for Source {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (index, name, alias, channels, volume, muted) =
            <(u32, String, String, u16, Vec<u32>, bool)>::get(i)?;
        Some(Self {
            index,
            name,
            alias,
            channels,
            volume,
            muted,
        })
    }
}

impl Arg for Source {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ussqaub)\0") }
    }
}

impl From<&SourceInfo<'_>> for Source {
    fn from(value: &SourceInfo<'_>) -> Self {
        let name_opt = &value.name;
        let alias_opt = &value.description;
        let name: String;
        let alias: String;
        if name_opt.is_none() {
            name = String::from("");
        } else {
            name = String::from(name_opt.clone().unwrap());
        }
        if alias_opt.is_none() {
            alias = String::from("");
        } else {
            alias = String::from(alias_opt.clone().unwrap());
        }
        let index = value.index;
        let channels = value.channel_map.len() as u16;
        let mut volume = vec![0; channels as usize];
        for i in 0..channels as usize {
            unsafe { *volume.get_unchecked_mut(i) = value.volume.get()[i].0 };
        }
        let muted = value.mute;
        Self {
            index,
            name,
            alias,
            channels,
            volume,
            muted,
        }
    }
}

#[derive(Debug)]
pub struct Sink {
    index: u32,
    name: String,
    alias: String,
    channels: u16,
    volume: Vec<u32>,
    muted: bool,
}

impl Append for Sink {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(&self.index);
            i.append(&self.name);
            i.append(&self.alias);
            i.append(&self.channels);
            i.append(&self.volume);
            i.append(&self.muted);
        });
    }
}

impl<'a> Get<'a> for Sink {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (index, name, alias, channels, volume, muted) =
            <(u32, String, String, u16, Vec<u32>, bool)>::get(i)?;
        Some(Self {
            index,
            name,
            alias,
            channels,
            volume,
            muted,
        })
    }
}

impl Arg for Sink {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ussqaub)\0") }
    }
}

impl From<&SinkInfo<'_>> for Sink {
    fn from(value: &SinkInfo<'_>) -> Self {
        let name_opt = &value.name;
        let alias_opt = &value.description;
        let name: String;
        let alias: String;
        if name_opt.is_none() {
            name = String::from("");
        } else {
            name = String::from(name_opt.clone().unwrap());
        }
        if alias_opt.is_none() {
            alias = String::from("");
        } else {
            alias = String::from(alias_opt.clone().unwrap());
        }
        let index = value.index;
        let channels = value.channel_map.len() as u16;
        let mut volume = vec![0; channels as usize];
        for i in 0..channels as usize {
            unsafe { *volume.get_unchecked_mut(i) = value.volume.get()[i].0 };
        }
        let muted = value.mute;
        Self {
            index,
            name,
            alias,
            channels,
            volume,
            muted,
        }
    }
}

pub struct InputStream {
    index: u32,
    name: String,
    application_name: String,
    sink_index: u32,
    channels: u16,
    volume: Vec<u32>,
    muted: bool,
}

impl Append for InputStream {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(&self.index);
            i.append(&self.name);
            i.append(&self.application_name);
            i.append(&self.sink_index);
            i.append(&self.channels);
            i.append(&self.volume);
            i.append(&self.muted);
        });
    }
}

impl<'a> Get<'a> for InputStream {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (index, name, application_name, sink_index, channels, volume, muted) =
            <(u32, String, String, u32, u16, Vec<u32>, bool)>::get(i)?;
        Some(Self {
            index,
            name,
            application_name,
            sink_index,
            channels,
            volume,
            muted,
        })
    }
}

impl Arg for InputStream {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ussuqaub)\0") }
    }
}

impl From<&SinkInputInfo<'_>> for InputStream {
    fn from(value: &SinkInputInfo<'_>) -> Self {
        let name_opt = &value.name;
        let name: String;
        if name_opt.is_none() {
            name = String::from("");
        } else {
            name = String::from(name_opt.clone().unwrap());
        }
        let application_name = value
            .proplist
            .get_str("application.name")
            .unwrap_or_default();
        let sink_index = value.sink;
        let index = value.index;
        let channels = value.channel_map.len() as u16;
        let mut volume = vec![0; channels as usize];
        for i in 0..channels as usize {
            unsafe { *volume.get_unchecked_mut(i) = value.volume.get()[i].0 };
        }
        let muted = value.mute;
        Self {
            index,
            name,
            application_name,
            sink_index,
            channels,
            volume,
            muted,
        }
    }
}

pub struct OutputStream {
    index: u32,
    name: String,
    application_name: String,
    source_index: u32,
    channels: u16,
    volume: Vec<u32>,
    muted: bool,
}

impl Append for OutputStream {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(&self.index);
            i.append(&self.name);
            i.append(&self.application_name);
            i.append(&self.source_index);
            i.append(&self.channels);
            i.append(&self.volume);
            i.append(&self.muted);
        });
    }
}

impl<'a> Get<'a> for OutputStream {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (index, name, application_name, source_index, channels, volume, muted) =
            <(u32, String, String, u32, u16, Vec<u32>, bool)>::get(i)?;
        Some(Self {
            index,
            name,
            application_name,
            source_index,
            channels,
            volume,
            muted,
        })
    }
}

impl Arg for OutputStream {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ussuqaub)\0") }
    }
}

impl From<&SourceOutputInfo<'_>> for OutputStream {
    fn from(value: &SourceOutputInfo<'_>) -> Self {
        let name_opt = &value.name;
        let name: String;
        if name_opt.is_none() {
            name = String::from("");
        } else {
            name = String::from(name_opt.clone().unwrap());
        }
        let application_name = value
            .proplist
            .get_str("application.name")
            .unwrap_or_default();
        let sink_index = value.source;
        let index = value.index;
        let channels = value.channel_map.len() as u16;
        let mut volume = vec![0; channels as usize];
        for i in 0..channels as usize {
            unsafe { *volume.get_unchecked_mut(i) = value.volume.get()[i].0 };
        }
        let muted = value.mute;
        Self {
            index,
            name,
            application_name,
            source_index: sink_index,
            channels,
            volume,
            muted,
        }
    }
}

impl PulseServer {
    pub fn create(
        sender: Sender<Response>,
        receiver: Receiver<Request>,
    ) -> Result<Self, PulseError> {
        let mut proplist = Proplist::new().unwrap();
        proplist
            .set_str(pulse::proplist::properties::APPLICATION_NAME, "ReSet")
            .unwrap();

        let mainloop = Rc::new(RefCell::new(
            Mainloop::new().expect("Failed to create mainloop"),
        ));

        let context = Rc::new(RefCell::new(
            Context::new_with_proplist(mainloop.borrow().deref(), "ReSetContext", &proplist)
                .expect("Failed to create new context"),
        ));

        {
            let ml_ref = Rc::clone(&mainloop);
            let context_ref = Rc::clone(&context);
            context
                .borrow_mut()
                .set_state_callback(Some(Box::new(move || {
                    let state = unsafe { (*context_ref.as_ptr()).get_state() };
                    match state {
                        pulse::context::State::Ready
                        | pulse::context::State::Failed
                        | pulse::context::State::Terminated => unsafe {
                            (*ml_ref.as_ptr()).signal(false);
                        },
                        _ => {}
                    }
                })));
        }

        context
            .borrow_mut()
            .connect(None, FlagSet::NOFLAGS, None)
            .expect("Failed to connect context");

        mainloop.borrow_mut().lock();
        mainloop
            .borrow_mut()
            .start()
            .expect("Failed to start mainloop");

        loop {
            match context.borrow().get_state() {
                pulse::context::State::Ready => {
                    break;
                }
                pulse::context::State::Failed | pulse::context::State::Terminated => {
                    mainloop.borrow_mut().unlock();
                    mainloop.borrow_mut().stop();
                    return Err(PulseError("Could not create context."));
                }
                _ => {
                    mainloop.borrow_mut().wait();
                }
            }
        }
        context.borrow_mut().set_state_callback(None);
        mainloop.borrow_mut().unlock();
        return Ok(Self {
            mainloop,
            context,
            sender,
            receiver,
        });
    }

    pub fn listen_to_messages(&mut self) {
        loop {
            let message = self.receiver.recv();
            if message.is_ok() {
                self.handle_message(message.unwrap());
            }
        }
    }

    // during development, as more get added => without causing compiler errors
    #[allow(unreachable_patterns)]
    pub fn handle_message(&self, message: Request) {
        match message {
            Request::ListSinks => self.get_sinks(),
            Request::ListSources => self.get_sources(),
            Request::ListInputStreams => self.get_input_streams(),
            Request::ListOutputStreams => self.get_output_streams(),
            Request::SetInputStreamMute(input_stream) => self.set_input_stream_mute(input_stream),
            Request::SetInputStreamVolume(input_stream) => {
                self.set_volume_of_input_stream(input_stream)
            }
            Request::SetSinkOfInputStream(inpu_stream, sink) => {
                self.set_sink_of_input_stream(inpu_stream, sink)
            }
            Request::SetOutputStreamMute(output_stream) => {
                self.set_output_stream_mute(output_stream)
            }
            Request::SetOutputStreamVolume(output_stream) => {
                self.set_volume_of_output_stream(output_stream)
            }
            Request::SetSourceOfOutputStream(output_stream, sink) => {
                self.set_source_of_output_stream(output_stream, sink)
            }
            Request::SetSinkVolume(sink) => self.set_sink_volume(sink),
            Request::SetSinkMute(sink) => self.set_sink_mute(sink),
            Request::SetDefaultSink(sink) => self.set_default_sink(sink),
            Request::SetSourceVolume(source) => self.set_source_volume(source),
            Request::SetSourceMute(source) => self.set_source_mute(source),
            _ => {}
        }
    }

    pub fn get_sinks(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let sinks = Rc::new(RefCell::new(Vec::new()));
        let sinks_ref = sinks.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.get_sink_info_list(move |result| match result {
            ListResult::Item(item) => {
                sinks_ref.borrow_mut().push(item.into());
            }
            ListResult::Error => unsafe {
                (*ml_ref.as_ptr()).signal(true);
            },
            ListResult::End => unsafe {
                (*ml_ref.as_ptr()).signal(false);
            },
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::Sinks(sinks.take()));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn get_sources(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let sources: Rc<RefCell<Vec<Source>>> = Rc::new(RefCell::new(Vec::new()));
        let sources_ref = sources.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.get_source_info_list(move |result| match result {
            ListResult::Item(item) => {
                sources_ref.borrow_mut().push(item.into());
            }
            ListResult::Error => unsafe {
                (*ml_ref.as_ptr()).signal(true);
            },
            ListResult::End => unsafe {
                (*ml_ref.as_ptr()).signal(false);
            },
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::Sources(sources.take()));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_sink_volume(&self, sink: Sink) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let mut channel_volume = ChannelVolumes::default();
        let channel_volume_slice = channel_volume.get_mut();
        let ml_ref = Rc::clone(&self.mainloop);
        for i in 0..sink.channels as usize {
            unsafe { channel_volume_slice[i] = Volume(*sink.volume.get_unchecked(i)) }
        }
        let result = introspector.set_sink_volume_by_index(
            sink.index,
            &channel_volume,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_sink_mute(&self, sink: Sink) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_sink_mute_by_index(
            sink.index,
            !sink.muted,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_source_volume(&self, source: Source) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let mut channel_volume = ChannelVolumes::default();
        let channel_volume_slice = channel_volume.get_mut();
        let ml_ref = Rc::clone(&self.mainloop);
        for i in 0..source.channels as usize {
            unsafe { channel_volume_slice[i] = Volume(*source.volume.get_unchecked(i)) }
        }
        let result = introspector.set_source_volume_by_index(
            source.index,
            &channel_volume,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_source_mute(&self, source: Source) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_source_mute_by_index(
            source.index,
            !source.muted,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_default_sink(&self, sink: Sink) {
        self.mainloop.borrow_mut().lock();
        let ml_ref = Rc::clone(&self.mainloop);
        let result =
            self.context
                .borrow_mut()
                .set_default_sink(&sink.name, move |error: bool| unsafe {
                    (*ml_ref.as_ptr()).signal(!error);
                });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn get_input_streams(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let input_streams = Rc::new(RefCell::new(Vec::new()));
        let input_stream = input_streams.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.get_sink_input_info_list(move |result| match result {
            ListResult::Item(item) => {
                input_stream.borrow_mut().push(item.into());
            }
            ListResult::Error => unsafe {
                (*ml_ref.as_ptr()).signal(true);
            },
            ListResult::End => unsafe {
                (*ml_ref.as_ptr()).signal(false);
            },
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self
            .sender
            .send(Response::InputStreams(input_streams.take()));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_sink_of_input_stream(&self, input_stream: InputStream, sink: Sink) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.move_sink_input_by_index(
            input_stream.index,
            sink.index,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_volume_of_input_stream(&self, input_stream: InputStream) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let mut channel_volume = ChannelVolumes::default();
        let channel_volume_slice = channel_volume.get_mut();
        let ml_ref = Rc::clone(&self.mainloop);
        for i in 0..input_stream.channels as usize {
            unsafe { channel_volume_slice[i] = Volume(*input_stream.volume.get_unchecked(i)) }
        }
        let result = introspector.set_sink_input_volume(
            input_stream.index,
            &channel_volume,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_input_stream_mute(&self, input_stream: InputStream) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_sink_input_mute(
            input_stream.index,
            !input_stream.muted,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn get_output_streams(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let output_streams = Rc::new(RefCell::new(Vec::new()));
        let output_stream_ref = output_streams.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.get_source_output_info_list(move |result| match result {
            ListResult::Item(item) => {
                output_stream_ref.borrow_mut().push(item.into());
            }
            ListResult::Error => unsafe {
                (*ml_ref.as_ptr()).signal(true);
            },
            ListResult::End => unsafe {
                (*ml_ref.as_ptr()).signal(false);
            },
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self
            .sender
            .send(Response::OutputStreams(output_streams.take()));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_source_of_output_stream(&self, output_stream: OutputStream, source: Source) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.move_source_output_by_index(
            output_stream.index,
            source.index,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_volume_of_output_stream(&self, output_stream: OutputStream) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let mut channel_volume = ChannelVolumes::default();
        let channel_volume_slice = channel_volume.get_mut();
        let ml_ref = Rc::clone(&self.mainloop);
        for i in 0..output_stream.channels as usize {
            unsafe { channel_volume_slice[i] = Volume(*output_stream.volume.get_unchecked(i)) }
        }
        let result = introspector.set_source_output_volume(
            output_stream.index,
            &channel_volume,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_output_stream_mute(&self, output_stream: OutputStream) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_source_output_mute(
            output_stream.index,
            !output_stream.muted,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(Response::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }
}
