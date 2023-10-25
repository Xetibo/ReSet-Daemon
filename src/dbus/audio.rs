use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{
        Arc,
    },
};

use tokio::sync::mpsc::{Receiver, Sender};

use dbus::{
    arg::{self, Append, Arg, ArgType, Get},
    Signature,
};
use pulse::{
    self,
    callbacks::ListResult,
    context::{
        introspect::{SinkInfo, SourceInfo},
        Context, FlagSet,
    },
    mainloop::{standard::IterateResult, threaded::Mainloop},
    operation::{Operation, State},
    proplist::Proplist,
};

use super::reset_dbus::Message;

pub struct PulseServer {
    mainloop: Arc<RefCell<Mainloop>>,
    context: Arc<RefCell<Context>>,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

#[derive(Debug)]
pub struct PulseError {
    message: &'static str,
}

pub struct Source {
    index: u32,
    name: String,
    channels: u8,
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
        let index = <u32>::get(i)?;
        let name = <String>::get(i)?;
        let channels = <u8>::get(i)?;
        let volume = <u32>::get(i)?;
        let muted = <bool>::get(i)?;
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
        unsafe { Signature::from_slice_unchecked("(isu)\0") }
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
        let channels = value.channel_map.len();
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

pub struct Sink {
    index: u32,
    name: String,
    channels: u8,
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
        let index = <u32>::get(i)?;
        let name = <String>::get(i)?;
        let channels = <u8>::get(i)?;
        let volume = <u32>::get(i)?;
        let muted = <bool>::get(i)?;
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
        unsafe { Signature::from_slice_unchecked("(isu)\0") }
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
        let channels = value.channel_map.len();
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
        sender: Sender<Message>,
        receiver: Receiver<Message>,
    ) -> Result<Self, PulseError> {
        let mut proplist = Proplist::new().unwrap();
        proplist
            .set_str(pulse::proplist::properties::APPLICATION_NAME, "ReSet")
            .unwrap();

        let mainloop = Arc::new(RefCell::new(
            Mainloop::new().expect("Failed to create mainloop"),
        ));

        let context = Arc::new(RefCell::new(
            Context::new_with_proplist(mainloop.borrow().deref(), "ReSetContext", &proplist)
                .expect("Failed to create new context"),
        ));

        {
            let ml_ref = Arc::clone(&mainloop);
            let context_ref = Arc::clone(&context);
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
                    return Err(PulseError {
                        message: "Could not create context.",
                    });
                }
                _ => {
                    mainloop.borrow_mut().wait();
                }
            }
        }
        context.borrow_mut().set_state_callback(None);
        return Ok(Self {
            mainloop,
            context,
            sender,
            receiver,
        });
    }

    pub fn get_sinks(&self) -> Vec<Sink> {
        let introspector = self.context.borrow().introspect();
        let sinks = Rc::new(RefCell::new(Vec::new()));
        let sinks_ref = sinks.clone();
        let result = introspector.get_sink_info_list(move |result| match result {
            ListResult::Item(item) => {
                sinks_ref.borrow_mut().push(item.into());
            }
            ListResult::Error => {}
            ListResult::End => {}
        });
        loop {
            match result.get_state() {
                pulse::operation::State::Done => {
                    return sinks.take();
                }
                pulse::operation::State::Running => {
                    self.mainloop.borrow_mut().wait();
                }
                pulse::operation::State::Cancelled => {
                    self.mainloop.borrow_mut().unlock();
                    self.mainloop.borrow_mut().stop();
                    return Vec::new();
                }
            }
        }
    }

    pub fn get_sources(&self) {
        let introspector = self.context.borrow().introspect();
        let sources: Rc<RefCell<Vec<Source>>> = Rc::new(RefCell::new(Vec::new()));
        let sources_ref = sources.clone();
        let result = introspector.get_source_info_list(move |result| match result {
            ListResult::Item(item) => {
                sources_ref.borrow_mut().push(item.into());
            }
            ListResult::Error => {}
            ListResult::End => {}
        });
        loop {
            match result.get_state() {
                pulse::operation::State::Done => {
                    break;
                }
                pulse::operation::State::Running => {
                    self.mainloop.borrow_mut().wait();
                }
                pulse::operation::State::Cancelled => {
                    self.mainloop.borrow_mut().unlock();
                    self.mainloop.borrow_mut().stop();
                    return;
                }
            }
        }
    }
}
