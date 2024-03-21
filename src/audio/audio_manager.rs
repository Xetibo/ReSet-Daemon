use std::sync::Arc;

use std::{cell::RefCell, ops::Deref, rc::Rc};

use crossbeam::channel::{Receiver, Sender};
use dbus::channel::Sender as dbus_sender;
use dbus::nonblock::SyncConnection;
use dbus::{Message, Path};
use pulse::context::introspect::Introspector;
use pulse::context::subscribe::{InterestMaskSet, Operation};
use pulse::def::Retval;
use pulse::mainloop::api::Mainloop as mainloop_api;
use pulse::volume::{ChannelVolumes, Volume};
use pulse::{
    callbacks::ListResult,
    context::{Context, FlagSet},
    mainloop::threaded::Mainloop,
    proplist::Proplist,
};
use re_set_lib::audio::audio_structures::{InputStream, OutputStream, Sink, Source};
use re_set_lib::{
    utils::macros::ErrorLevel,
    {write_log_to_file, ERROR},
};

use crate::utils::AUDIO;
use crate::{AudioRequest, AudioResponse};

pub struct PulseServer {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
    sender: Sender<AudioResponse>,
    receiver: Receiver<AudioRequest>,
}

#[derive(Debug)]
pub struct PulseError(pub &'static str);

impl PulseServer {
    pub fn create(
        sender: Sender<AudioResponse>,
        receiver: Receiver<AudioRequest>,
        connection: Arc<SyncConnection>,
    ) -> Result<Self, PulseError> {
        let mut proplist = Proplist::new().unwrap();
        proplist
            .set_str(pulse::proplist::properties::APPLICATION_NAME, AUDIO)
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
            .connect(None, FlagSet::NOAUTOSPAWN, None)
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

        let mut mask = InterestMaskSet::empty();
        mask.insert(InterestMaskSet::SINK);
        mask.insert(InterestMaskSet::SOURCE);
        mask.insert(InterestMaskSet::SINK_INPUT);
        mask.insert(InterestMaskSet::SOURCE_OUTPUT);

        context.borrow_mut().subscribe(mask, |_| {});
        let connection_ref = connection.clone();
        {
            let mut borrow = context.borrow_mut();
            let introspector = borrow.introspect();
            borrow.set_subscribe_callback(Some(Box::new(move |facility, operation, index| {
                let connection = connection_ref.clone();
                let connection_sink = connection_ref.clone();
                let connection_source = connection_ref.clone();
                let connection_input_stream = connection_ref.clone();
                let connection_output_stream = connection_ref.clone();
                let operation = operation.unwrap();
                let facility = facility.unwrap();
                match facility {
                    pulse::context::subscribe::Facility::Sink => {
                        if operation == Operation::Removed {
                            handle_sink_removed(&connection_ref, index);
                            return;
                        }
                        introspector.get_sink_info_by_index(index, move |result| match result {
                            ListResult::Item(sink) => {
                                handle_sink_events(&connection_sink, Sink::from(sink), operation);
                            }
                            ListResult::Error => {
                                ERROR!(
                                    "/tmp/reset_daemon_log",
                                    "Could not get sink info",
                                    ErrorLevel::PartialBreakage
                                );
                            }
                            ListResult::End => (),
                        });
                    }
                    pulse::context::subscribe::Facility::Source => {
                        if operation == Operation::Removed {
                            handle_source_removed(&connection, index);
                            return;
                        }
                        introspector.get_source_info_by_index(index, move |result| match result {
                            ListResult::Item(source) => {
                                handle_source_events(
                                    &connection_source,
                                    Source::from(source),
                                    operation,
                                );
                            }
                            ListResult::Error => {
                                ERROR!(
                                    "/tmp/reset_daemon_log",
                                    "Could not get source info",
                                    ErrorLevel::PartialBreakage
                                );
                            }
                            ListResult::End => (),
                        });
                    }
                    pulse::context::subscribe::Facility::SinkInput => {
                        if operation == Operation::Removed {
                            handle_input_stream_removed(&connection, index);
                            return;
                        }
                        introspector.get_sink_input_info(index, move |result| match result {
                            ListResult::Item(input_stream) => {
                                handle_input_stream_events(
                                    &connection_input_stream,
                                    InputStream::from(input_stream),
                                    operation,
                                );
                            }
                            ListResult::Error => {
                                ERROR!(
                                    "/tmp/reset_daemon_log",
                                    "Could not get output stream info",
                                    ErrorLevel::PartialBreakage
                                );
                            }
                            ListResult::End => (),
                        });
                    }
                    pulse::context::subscribe::Facility::SourceOutput => {
                        if operation == Operation::Removed {
                            handle_output_stream_removed(&connection, index);
                            return;
                        }
                        introspector.get_source_output_info(index, move |result| match result {
                            ListResult::Item(output_stream) => {
                                handle_output_stream_events(
                                    &connection_output_stream,
                                    OutputStream::from(output_stream),
                                    operation,
                                );
                            }
                            ListResult::Error => {
                                ERROR!(
                                    "/tmp/reset_daemon_log",
                                    "Could not get input stream info",
                                    ErrorLevel::PartialBreakage
                                );
                            }
                            ListResult::End => (),
                        });
                    }
                    _ => (),
                }
            })));
        }

        context.borrow_mut().set_state_callback(None);
        mainloop.borrow_mut().unlock();
        Ok(Self {
            mainloop,
            context,
            sender,
            receiver,
        })
    }

    pub fn listen_to_messages(&mut self) {
        loop {
            let message = self.receiver.recv();
            if let Ok(message) = message {
                self.handle_message(message);
            }
        }
    }

    pub fn handle_message(&self, message: AudioRequest) {
        match message {
            AudioRequest::ListSinks => self.get_sinks(),
            AudioRequest::GetDefaultSink => self.get_default_sink(),
            AudioRequest::GetDefaultSinkName => self.get_default_sink_name(),
            AudioRequest::ListSources => self.get_sources(),
            AudioRequest::GetDefaultSource => self.get_default_source(),
            AudioRequest::GetDefaultSourceName => self.get_default_source_name(),
            AudioRequest::ListInputStreams => self.get_input_streams(),
            AudioRequest::ListOutputStreams => self.get_output_streams(),
            AudioRequest::SetInputStreamMute(index, muted) => {
                self.set_input_stream_mute(index, muted)
            }
            AudioRequest::SetInputStreamVolume(index, channels, volume) => {
                self.set_volume_of_input_stream(index, channels, volume)
            }
            AudioRequest::SetSinkOfInputStream(input_stream, sink) => {
                self.set_sink_of_input_stream(input_stream, sink)
            }
            AudioRequest::SetOutputStreamMute(index, muted) => {
                self.set_output_stream_mute(index, muted)
            }
            AudioRequest::SetOutputStreamVolume(index, channels, volume) => {
                self.set_volume_of_output_stream(index, channels, volume)
            }
            AudioRequest::SetSourceOfOutputStream(output_stream, sink) => {
                self.set_source_of_output_stream(output_stream, sink)
            }
            AudioRequest::SetSinkVolume(index, channels, volume) => {
                self.set_sink_volume(index, channels, volume)
            }
            AudioRequest::SetSinkMute(index, muted) => self.set_sink_mute(index, muted),
            AudioRequest::SetDefaultSink(sink) => self.set_default_sink(sink),
            AudioRequest::SetSourceVolume(index, channels, volume) => {
                self.set_source_volume(index, channels, volume)
            }
            AudioRequest::SetSourceMute(index, muted) => self.set_source_mute(index, muted),
            AudioRequest::SetDefaultSource(source) => self.set_default_source(source),
            AudioRequest::ListCards => self.get_cards(),
            AudioRequest::SetCardProfileOfDevice(device_index, profile_name) => {
                self.set_card_profile_of_device(device_index, profile_name)
            }
            AudioRequest::StopListener => self.stop_listener(),
        }
    }

    pub fn stop_listener(&self) {
        self.mainloop.borrow_mut().lock();
        self.mainloop.borrow_mut().stop();
        self.mainloop.borrow_mut().quit(Retval(0));
    }

    pub fn get_default_sink(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let sink = Rc::new(RefCell::new(Sink::default()));
        let sink_ref = sink.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let sink_name = self.no_lock_get_default_sink_name(&introspector);
        if sink_name.borrow().is_empty() {
            let _ = self.sender.send(AudioResponse::Error);
            self.mainloop.borrow_mut().unlock();
            return;
        }
        let result =
            introspector.get_sink_info_by_name(
                sink_name.take().as_str(),
                move |result| match result {
                    ListResult::Item(item) => {
                        sink_ref.replace(item.into());
                    }
                    ListResult::Error => unsafe {
                        ERROR!(
                            "/tmp/reset_daemon_log",
                            "Could not get sink info",
                            ErrorLevel::PartialBreakage
                        );
                        (*ml_ref.as_ptr()).signal(true);
                    },
                    ListResult::End => unsafe {
                        (*ml_ref.as_ptr()).signal(false);
                    },
                },
            );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let res = self.sender.send(AudioResponse::DefaultSink(sink.take()));
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Could not get default sink",
                ErrorLevel::PartialBreakage
            );
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn get_default_sink_name(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let source_name = self.no_lock_get_default_sink_name(&introspector);
        if source_name.borrow().is_empty() {
            let res = self.sender.send(AudioResponse::Error);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get default sink name",
                    ErrorLevel::PartialBreakage
                );
            }
            self.mainloop.borrow_mut().unlock();
            return;
        }
        let res = self
            .sender
            .send(AudioResponse::DefaultSinkName(source_name.take()));
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Could not get default sink name",
                ErrorLevel::PartialBreakage
            );
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn no_lock_get_default_sink_name(
        &self,
        introspector: &Introspector,
    ) -> Rc<RefCell<String>> {
        let ml_ref_info = Rc::clone(&self.mainloop);
        let sink_name = Rc::new(RefCell::new(String::from("")));
        let sink_name_ref = sink_name.clone();
        let result = introspector.get_server_info(move |result| {
            if result.default_sink_name.is_some() {
                let mut borrow = sink_name_ref.borrow_mut();
                *borrow = String::from(result.default_sink_name.clone().unwrap());
                unsafe {
                    (*ml_ref_info.as_ptr()).signal(false);
                }
            }
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        sink_name
    }

    pub fn get_default_source_name(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let source_name = self.no_lock_get_default_source_name(&introspector);
        if source_name.borrow().is_empty() {
            let res = self.sender.send(AudioResponse::Error);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get default source name",
                    ErrorLevel::PartialBreakage
                );
            }
            self.mainloop.borrow_mut().unlock();
            return;
        }
        let res = self
            .sender
            .send(AudioResponse::DefaultSourceName(source_name.take()));
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Could not get default source name",
                ErrorLevel::PartialBreakage
            );
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn no_lock_get_default_source_name(
        &self,
        introspector: &Introspector,
    ) -> Rc<RefCell<String>> {
        let ml_ref_info = Rc::clone(&self.mainloop);
        let source_name = Rc::new(RefCell::new(String::from("")));
        let source_name_ref = source_name.clone();
        let result = introspector.get_server_info(move |result| {
            if result.default_source_name.is_some() {
                let mut borrow = source_name_ref.borrow_mut();
                *borrow = String::from(result.default_source_name.clone().unwrap());
                unsafe {
                    (*ml_ref_info.as_ptr()).signal(false);
                }
            }
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        source_name
    }

    pub fn get_default_source(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let source = Rc::new(RefCell::new(Source::default()));
        let source_ref = source.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let source_name = self.no_lock_get_default_source_name(&introspector);
        if source_name.borrow().is_empty() {
            let _ = self.sender.send(AudioResponse::Error);
            self.mainloop.borrow_mut().unlock();
            return;
        }
        let result =
            introspector.get_source_info_by_name(source_name.take().as_str(), move |result| {
                match result {
                    ListResult::Item(item) => {
                        source_ref.replace(item.into());
                    }
                    ListResult::Error => unsafe {
                        ERROR!(
                            "/tmp/reset_daemon_log",
                            "Could not get default source",
                            ErrorLevel::PartialBreakage
                        );
                        (*ml_ref.as_ptr()).signal(true);
                    },
                    ListResult::End => unsafe {
                        (*ml_ref.as_ptr()).signal(false);
                    },
                }
            });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let res = self
            .sender
            .send(AudioResponse::DefaultSource(source.take()));
        if res.is_err() {
            ERROR!(
                "/tmp/reset_daemon_log",
                "Could not get default source",
                ErrorLevel::PartialBreakage
            );
        }
        self.mainloop.borrow_mut().unlock();
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
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get sinks",
                    ErrorLevel::PartialBreakage
                );
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
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get sources",
                    ErrorLevel::PartialBreakage
                );
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

    pub fn set_sink_volume(&self, index: u32, channels: u16, volume: u32) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let mut channel_volume = ChannelVolumes::default();
        channel_volume.set_len(channels as u8);
        channel_volume.set(channels as u8, Volume(volume));
        let ml_ref = Rc::clone(&self.mainloop);
        let _result = introspector.set_sink_volume_by_index(
            index,
            &channel_volume,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_sink_mute(&self, index: u32, muted: bool) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_sink_mute_by_index(
            index,
            muted,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_source_volume(&self, index: u32, channels: u16, volume: u32) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let mut channel_volume = ChannelVolumes::default();
        channel_volume.set_len(channels as u8);
        channel_volume.set(channels as u8, Volume(volume));
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_source_volume_by_index(
            index,
            &channel_volume,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_source_mute(&self, index: u32, muted: bool) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_source_mute_by_index(
            index,
            muted,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_default_sink(&self, sink_name: String) {
        self.mainloop.borrow_mut().lock();
        let mut context = self.context.borrow_mut();
        let sink: Rc<RefCell<Sink>> = Rc::new(RefCell::new(Sink::default()));
        let sink_ref = sink.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let ml_ref_response = Rc::clone(&self.mainloop);
        let result = context.set_default_sink(&sink_name, move |error: bool| unsafe {
            (*ml_ref.as_ptr()).signal(!error);
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let introspector = context.introspect();
        let result = introspector.get_sink_info_by_name(&sink_name, move |result| match result {
            ListResult::Item(item) => {
                sink_ref.replace(item.into());
            }
            ListResult::Error => unsafe {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not set default sink",
                    ErrorLevel::PartialBreakage
                );
                (*ml_ref_response.as_ptr()).signal(true);
            },
            ListResult::End => unsafe {
                (*ml_ref_response.as_ptr()).signal(false);
            },
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(AudioResponse::DefaultSink(sink.take()));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_default_source(&self, source_name: String) {
        self.mainloop.borrow_mut().lock();
        let mut context = self.context.borrow_mut();
        let source: Rc<RefCell<Source>> = Rc::new(RefCell::new(Source::default()));
        let source_ref = source.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let ml_ref_response = Rc::clone(&self.mainloop);
        let result = context.set_default_source(&source_name, move |error: bool| unsafe {
            (*ml_ref.as_ptr()).signal(!error);
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let introspector = context.introspect();
        let result =
            introspector.get_source_info_by_name(&source_name, move |result| match result {
                ListResult::Item(item) => {
                    source_ref.replace(item.into());
                }
                ListResult::Error => unsafe {
                    ERROR!(
                        "/tmp/reset_daemon_log",
                        "Could not set default source",
                        ErrorLevel::PartialBreakage
                    );
                    (*ml_ref_response.as_ptr()).signal(true);
                },
                ListResult::End => unsafe {
                    (*ml_ref_response.as_ptr()).signal(false);
                },
            });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self
            .sender
            .send(AudioResponse::DefaultSource(source.take()));
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
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get input streams",
                    ErrorLevel::PartialBreakage
                );
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

    pub fn set_sink_of_input_stream(&self, input_stream: u32, sink: u32) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.move_sink_input_by_index(
            input_stream,
            sink,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_volume_of_input_stream(&self, index: u32, channels: u16, volume: u32) {
        self.mainloop.borrow_mut().lock();
        let ml_ref = Rc::clone(&self.mainloop);
        let mut introspector = self.context.borrow_mut().introspect();
        let mut channel_volume = ChannelVolumes::default();
        channel_volume.set_len(channels as u8);
        channel_volume.set(channels as u8, Volume(volume));
        let result = introspector.set_sink_input_volume(
            index,
            &channel_volume,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_input_stream_mute(&self, index: u32, muted: bool) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_sink_input_mute(
            index,
            muted,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
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
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get output streams",
                    ErrorLevel::PartialBreakage
                );
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

    pub fn set_source_of_output_stream(&self, output_stream: u32, source: u32) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.move_source_output_by_index(
            output_stream,
            source,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_volume_of_output_stream(&self, index: u32, channels: u16, volume: u32) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let mut channel_volume = ChannelVolumes::default();
        channel_volume.set_len(channels as u8);
        channel_volume.set(channels as u8, Volume(volume));
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_source_output_volume(
            index,
            &channel_volume,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_output_stream_mute(&self, index: u32, muted: bool) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow_mut().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_source_output_mute(
            index,
            muted,
            Some(Box::new(move |error| unsafe {
                (*ml_ref.as_ptr()).signal(!error);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }

    pub fn get_cards(&self) {
        self.mainloop.borrow_mut().lock();
        let introspector = self.context.borrow().introspect();
        let cards = Rc::new(RefCell::new(Vec::new()));
        let cards_ref = cards.clone();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.get_card_info_list(move |result| match result {
            ListResult::Item(item) => {
                cards_ref.borrow_mut().push(item.into());
            }
            ListResult::Error => unsafe {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get profile cards",
                    ErrorLevel::PartialBreakage
                );
                (*ml_ref.as_ptr()).signal(false);
            },
            ListResult::End => unsafe {
                (*ml_ref.as_ptr()).signal(false);
            },
        });
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        let _ = self.sender.send(AudioResponse::Cards(cards.take()));
        self.mainloop.borrow_mut().unlock();
    }

    pub fn set_card_profile_of_device(&self, device_index: u32, profile_name: String) {
        self.mainloop.borrow_mut().lock();
        let mut introspector = self.context.borrow().introspect();
        let ml_ref = Rc::clone(&self.mainloop);
        let result = introspector.set_card_profile_by_index(
            device_index,
            &profile_name,
            Some(Box::new(move |_| unsafe {
                (*ml_ref.as_ptr()).signal(false);
            })),
        );
        while result.get_state() != pulse::operation::State::Done {
            self.mainloop.borrow_mut().wait();
        }
        self.mainloop.borrow_mut().unlock();
    }
}

fn handle_sink_events(conn: &Arc<SyncConnection>, sink: Sink, operation: Operation) {
    match operation {
        Operation::New => {
            let msg = Message::signal(
                &Path::from(DBUS_PATH!()),
                &AUDIO.into(),
                &"SinkAdded".into(),
            )
            .append1(sink);
            let res = conn.send(msg);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get send message",
                    ErrorLevel::PartialBreakage
                );
            }
        }
        Operation::Changed => {
            let msg = Message::signal(
                &Path::from(DBUS_PATH!()),
                &AUDIO.into(),
                &"SinkChanged".into(),
            )
            .append1(sink);
            let res = conn.send(msg);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get send message",
                    ErrorLevel::PartialBreakage
                );
            }
        }
        Operation::Removed => (),
    }
}

fn handle_sink_removed(conn: &Arc<SyncConnection>, index: u32) {
    let msg = Message::signal(
        &Path::from(DBUS_PATH!()),
        &AUDIO.into(),
        &"SinkRemoved".into(),
    )
    .append1(index);
    let res = conn.send(msg);
    if res.is_err() {
        ERROR!(
            "/tmp/reset_daemon_log",
            "Could not get send message",
            ErrorLevel::PartialBreakage
        );
    }
}

fn handle_source_events(conn: &Arc<SyncConnection>, source: Source, operation: Operation) {
    match operation {
        Operation::New => {
            let msg = Message::signal(
                &Path::from(DBUS_PATH!()),
                &AUDIO.into(),
                &"SourceAdded".into(),
            )
            .append1(source);
            let res = conn.send(msg);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get send message",
                    ErrorLevel::PartialBreakage
                );
            }
        }
        Operation::Changed => {
            let msg = Message::signal(
                &Path::from(DBUS_PATH!()),
                &AUDIO.into(),
                &"SourceChanged".into(),
            )
            .append1(source);
            let res = conn.send(msg);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get send message",
                    ErrorLevel::PartialBreakage
                );
            }
        }
        Operation::Removed => (),
    }
}

fn handle_source_removed(conn: &Arc<SyncConnection>, index: u32) {
    let msg = Message::signal(
        &Path::from(DBUS_PATH!()),
        &AUDIO.into(),
        &"SourceRemoved".into(),
    )
    .append1(index);
    let res = conn.send(msg);
    if res.is_err() {
        ERROR!(
            "/tmp/reset_daemon_log",
            "Could not get send message",
            ErrorLevel::PartialBreakage
        );
    }
}

fn handle_input_stream_events(
    conn: &Arc<SyncConnection>,
    input_stream: InputStream,
    operation: Operation,
) {
    match operation {
        Operation::New => {
            let msg = Message::signal(
                &Path::from(DBUS_PATH!()),
                &AUDIO.into(),
                &"InputStreamAdded".into(),
            )
            .append1(input_stream);
            let res = conn.send(msg);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get send message",
                    ErrorLevel::PartialBreakage
                );
            }
        }
        Operation::Changed => {
            let msg = Message::signal(
                &Path::from(DBUS_PATH!()),
                &AUDIO.into(),
                &"InputStreamChanged".into(),
            )
            .append1(input_stream);
            let res = conn.send(msg);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get send message",
                    ErrorLevel::PartialBreakage
                );
            }
        }
        Operation::Removed => (),
    }
}

fn handle_input_stream_removed(conn: &Arc<SyncConnection>, index: u32) {
    let msg = Message::signal(
        &Path::from(DBUS_PATH!()),
        &AUDIO.into(),
        &"InputStreamRemoved".into(),
    )
    .append1(index);
    let res = conn.send(msg);
    if res.is_err() {
        ERROR!(
            "/tmp/reset_daemon_log",
            "Could not get send message",
            ErrorLevel::PartialBreakage
        );
    }
}

fn handle_output_stream_events(
    conn: &Arc<SyncConnection>,
    output_stream: OutputStream,
    operation: Operation,
) {
    match operation {
        Operation::New => {
            let msg = Message::signal(
                &Path::from(DBUS_PATH!()),
                &AUDIO.into(),
                &"OutputStreamAdded".into(),
            )
            .append1(output_stream);
            let res = conn.send(msg);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get send message",
                    ErrorLevel::PartialBreakage
                );
            }
        }
        Operation::Changed => {
            let msg = Message::signal(
                &Path::from(DBUS_PATH!()),
                &AUDIO.into(),
                &"OutputStreamChanged".into(),
            )
            .append1(output_stream);
            let res = conn.send(msg);
            if res.is_err() {
                ERROR!(
                    "/tmp/reset_daemon_log",
                    "Could not get send message",
                    ErrorLevel::PartialBreakage
                );
            }
        }
        Operation::Removed => (),
    }
}

fn handle_output_stream_removed(conn: &Arc<SyncConnection>, index: u32) {
    let msg = Message::signal(
        &Path::from(DBUS_PATH!()),
        &AUDIO.into(),
        &"OutputStreamRemoved".into(),
    )
    .append1(index);
    let res = conn.send(msg);
    if res.is_err() {
        ERROR!(
            "/tmp/reset_daemon_log",
            "Could not get send message",
            ErrorLevel::PartialBreakage
        );
    }
}
