use std::{cell::RefCell, ops::Deref, rc::Rc};

use std::sync::mpsc::{Receiver, Sender};

use pulse::volume::{ChannelVolumes, Volume};
use pulse::{
    self,
    callbacks::ListResult,
    context::{Context, FlagSet},
    mainloop::threaded::Mainloop,
    proplist::Proplist,
};
use ReSet_Lib::audio::audio::{InputStream, OutputStream, Sink, Source};

use crate::reset_dbus::{AudioRequest, AudioResponse};

pub struct PulseServer {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
    sender: Sender<AudioResponse>,
    receiver: Receiver<AudioRequest>,
}

#[derive(Debug)]
pub struct PulseError(&'static str);

impl PulseServer {
    pub fn create(
        sender: Sender<AudioResponse>,
        receiver: Receiver<AudioRequest>,
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
    pub fn handle_message(&self, message: AudioRequest) {
        match message {
            AudioRequest::ListSinks => self.get_sinks(),
            AudioRequest::ListSources => self.get_sources(),
            AudioRequest::ListInputStreams => self.get_input_streams(),
            AudioRequest::ListOutputStreams => self.get_output_streams(),
            AudioRequest::SetInputStreamMute(input_stream) => {
                self.set_input_stream_mute(input_stream)
            }
            AudioRequest::SetInputStreamVolume(input_stream) => {
                self.set_volume_of_input_stream(input_stream)
            }
            AudioRequest::SetSinkOfInputStream(inpu_stream, sink) => {
                self.set_sink_of_input_stream(inpu_stream, sink)
            }
            AudioRequest::SetOutputStreamMute(output_stream) => {
                self.set_output_stream_mute(output_stream)
            }
            AudioRequest::SetOutputStreamVolume(output_stream) => {
                self.set_volume_of_output_stream(output_stream)
            }
            AudioRequest::SetSourceOfOutputStream(output_stream, sink) => {
                self.set_source_of_output_stream(output_stream, sink)
            }
            AudioRequest::SetSinkVolume(sink) => self.set_sink_volume(sink),
            AudioRequest::SetSinkMute(sink) => self.set_sink_mute(sink),
            AudioRequest::SetDefaultSink(sink) => self.set_default_sink(sink),
            AudioRequest::SetSourceVolume(source) => self.set_source_volume(source),
            AudioRequest::SetSourceMute(source) => self.set_source_mute(source),
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
        let _ = self.sender.send(AudioResponse::Sinks(sinks.take()));
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
        let _ = self.sender.send(AudioResponse::Sources(sources.take()));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
            .send(AudioResponse::InputStreams(input_streams.take()));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
            .send(AudioResponse::OutputStreams(output_streams.take()));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
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
        let _ = self.sender.send(AudioResponse::BoolResponse(true));
        self.mainloop.borrow_mut().unlock();
    }
}
