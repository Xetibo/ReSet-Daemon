use std::{cell::RefCell, ops::Deref, rc::Rc};

use std::sync::mpsc::{Receiver, Sender};

use dbus::{
    arg::{self, Append, Arg, ArgType, Get},
    Signature,
};
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

use super::reset_dbus::{Request, Response};

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
    channels: u16,
    volume: u32,
    muted: bool,
}

impl Append for Source {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(&self.index);
            i.append(&self.name);
            i.append(&self.channels);
            i.append(&self.volume);
            i.append(&self.muted);
        });
    }
}

impl<'a> Get<'a> for Source {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (index, name, channels, volume, muted) = <(u32, String, u16, u32, bool)>::get(i)?;
        Some(Source {
            index,
            name,
            channels,
            volume,
            muted,
        })
    }
}

impl Arg for Source {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(usqub)\0") }
    }
}

impl From<&SourceInfo<'_>> for Source {
    fn from(value: &SourceInfo<'_>) -> Self {
        let name_opt = &value.description;
        let name: String;
        if name_opt.is_none() {
            name = String::from("");
        } else {
            name = String::from(name_opt.clone().unwrap());
        }
        let index = value.index;
        let channels = value.channel_map.len() as u16;
        let volume = value.volume.get()[0].0;
        let muted = value.mute;
        Self {
            index,
            name,
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
    channels: u16,
    volume: u32,
    muted: bool,
}

impl Append for Sink {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(&self.index);
            i.append(&self.name);
            i.append(&self.channels);
            i.append(&self.volume);
            i.append(&self.muted);
        });
    }
}

impl<'a> Get<'a> for Sink {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (index, name, channels, volume, muted) = <(u32, String, u16, u32, bool)>::get(i)?;
        Some(Sink {
            index,
            name,
            channels,
            volume,
            muted,
        })
    }
}

impl Arg for Sink {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(usqub)\0") }
    }
}

impl From<&SinkInfo<'_>> for Sink {
    fn from(value: &SinkInfo<'_>) -> Self {
        let name_opt = &value.description;
        let name: String;
        if name_opt.is_none() {
            name = String::from("");
        } else {
            name = String::from(name_opt.clone().unwrap());
        }
        let index = value.index;
        let channels = value.channel_map.len() as u16;
        let volume = value.volume.get()[0].0;
        let muted = value.mute;
        Self {
            index,
            name,
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
            Request::SetSinkVolume(sink) => self.set_sink_volume(sink),
            Request::SetSinkMute(sink) => self.set_sink_mute(sink),
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
        let ml_ref = Rc::clone(&self.mainloop);
        channel_volume.set(sink.channels as u8, Volume(sink.volume));
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
        let ml_ref = Rc::clone(&self.mainloop);
        channel_volume.set(source.channels as u8, Volume(source.volume));
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
}
