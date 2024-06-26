use dbus_crossroads::Crossroads;
use re_set_lib::audio::audio_structures::{Card, InputStream, OutputStream, Sink, Source};

use crate::{
    utils::{AudioRequest, AudioResponse, AUDIO},
    DaemonData,
};

pub fn setup_audio_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    let token = cross.register(AUDIO, |c| {
        c.signal::<(Sink,), _>("SinkChanged", ("sink",));
        c.signal::<(Sink,), _>("SinkAdded", ("sink",));
        c.signal::<(u32,), _>("SinkRemoved", ("sink",));
        c.signal::<(Source,), _>("SourceChanged", ("source",));
        c.signal::<(Source,), _>("SourceAdded", ("source",));
        c.signal::<(u32,), _>("SourceRemoved", ("source",));
        c.signal::<(InputStream,), _>("InputStreamChanged", ("input_stream",));
        c.signal::<(InputStream,), _>("InputStreamAdded", ("input_stream",));
        c.signal::<(u32,), _>("InputStreamRemoved", ("input_stream",));
        c.signal::<(OutputStream,), _>("OutputStreamChanged", ("output_stream",));
        c.signal::<(OutputStream,), _>("OutputStreamAdded", ("output_stream",));
        c.signal::<(u32,), _>("OutputStreamRemoved", ("output_stream",));
        c.method_with_cr_async(
            "GetDefaultSink",
            (),
            ("default_sink",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                let receiver = data.audio_receiver.clone();
                async move {
                    let _ = sender.send(AudioRequest::GetDefaultSink);
                    let response = receiver.recv();
                    let sink: Option<Sink> = if let Ok(response) = response {
                        match response {
                            AudioResponse::DefaultSink(s) => Some(s),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let response: Result<(Sink,), dbus::MethodErr> = if let Some(sink) = sink {
                        Ok((sink,))
                    } else {
                        Err(dbus::MethodErr::failed("Could not get default sink"))
                    };
                    ctx.reply(response)
                }
            },
        );
        c.method_with_cr_async(
            "GetDefaultSinkName",
            (),
            ("sink_name",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                let receiver = data.audio_receiver.clone();
                async move {
                    let _ = sender.send(AudioRequest::GetDefaultSinkName);
                    let response = receiver.recv();
                    let sink_name = if let Ok(response) = response {
                        match response {
                            AudioResponse::DefaultSinkName(s) => s,
                            _ => String::from(""),
                        }
                    } else {
                        String::from("")
                    };
                    ctx.reply(Ok((sink_name,)))
                }
            },
        );
        c.method_with_cr_async(
            "GetDefaultSource",
            (),
            ("default_source",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                let receiver = data.audio_receiver.clone();
                async move {
                    let _ = sender.send(AudioRequest::GetDefaultSource);
                    let response = receiver.recv();
                    let source: Option<Source> = if let Ok(response) = response {
                        match response {
                            AudioResponse::DefaultSource(s) => Some(s),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let response: Result<(Source,), dbus::MethodErr> = if let Some(source) = source
                    {
                        Ok((source,))
                    } else {
                        Err(dbus::MethodErr::failed("Could not get default source"))
                    };
                    ctx.reply(response)
                }
            },
        );
        c.method_with_cr_async(
            "GetDefaultSourceName",
            (),
            ("source_name",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                let receiver = data.audio_receiver.clone();
                async move {
                    let _ = sender.send(AudioRequest::GetDefaultSourceName);
                    let response = receiver.recv();
                    let source_name = if let Ok(response) = response {
                        match response {
                            AudioResponse::DefaultSourceName(s) => s,
                            _ => String::from(""),
                        }
                    } else {
                        String::from("")
                    };
                    ctx.reply(Ok((source_name,)))
                }
            },
        );
        c.method_with_cr_async("ListSinks", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let sender = data.audio_sender.clone();
            let receiver = data.audio_receiver.clone();
            async move {
                let _ = sender.send(AudioRequest::ListSinks);
                let response = receiver.recv();
                let sinks: Vec<Sink> = if let Ok(response) = response {
                    match response {
                        AudioResponse::Sinks(s) => s,
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                ctx.reply(Ok((sinks,)))
            }
        });
        c.method_with_cr_async("ListSources", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let sender = data.audio_sender.clone();
            let receiver = data.audio_receiver.clone();
            async move {
                let _ = sender.send(AudioRequest::ListSources);
                let response = receiver.recv();
                let sources: Vec<Source> = if let Ok(response) = response {
                    match response {
                        AudioResponse::Sources(s) => s,
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                ctx.reply(Ok((sources,)))
            }
        });
        c.method_with_cr_async(
            "SetSinkVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetSinkVolume(index, channels, volume));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "SetSinkMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetSinkMute(index, muted));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "SetSourceVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetSourceVolume(index, channels, volume));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "SetSourceMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetSourceMute(index, muted));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "SetDefaultSink",
            ("sink",),
            ("sink",),
            move |mut ctx, cross, (sink,): (String,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                let receiver = data.audio_receiver.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetDefaultSink(sink));
                    let response = receiver.recv();
                    let result = if let Ok(AudioResponse::DefaultSink(response)) = response {
                        Ok((response,))
                    } else {
                        Err(dbus::MethodErr::failed("Could not get default sink"))
                    };
                    ctx.reply(result)
                }
            },
        );
        c.method_with_cr_async(
            "SetDefaultSource",
            ("source",),
            ("source",),
            move |mut ctx, cross, (source,): (String,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                let receiver = data.audio_receiver.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetDefaultSource(source));
                    let response = receiver.recv();
                    let result = if let Ok(AudioResponse::DefaultSource(response)) = response {
                        Ok((response,))
                    } else {
                        Err(dbus::MethodErr::failed("Could not get default source"))
                    };
                    ctx.reply(result)
                }
            },
        );
        c.method_with_cr_async(
            "ListInputStreams",
            (),
            ("input_streams",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                let receiver = data.audio_receiver.clone();
                async move {
                    let _ = sender.send(AudioRequest::ListInputStreams);
                    let response = receiver.recv();
                    let input_streams: Vec<InputStream> = if let Ok(response) = response {
                        match response {
                            AudioResponse::InputStreams(s) => s,
                            _ => Vec::new(),
                        }
                    } else {
                        Vec::new()
                    };
                    ctx.reply(Ok((input_streams,)))
                }
            },
        );
        c.method_with_cr_async(
            "SetSinkOfInputStream",
            ("input_stream", "sink"),
            (),
            move |mut ctx, cross, (input_stream, sink): (u32, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetSinkOfInputStream(input_stream, sink));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "SetInputStreamVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ =
                        sender.send(AudioRequest::SetInputStreamVolume(index, channels, volume));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "SetInputStreamMute",
            ("input_stream_index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetInputStreamMute(index, muted));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "ListOutputStreams",
            (),
            ("output_streams",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                let receiver = data.audio_receiver.clone();
                async move {
                    let _ = sender.send(AudioRequest::ListOutputStreams);
                    let response = receiver.recv();
                    let output_streams: Vec<OutputStream> = if let Ok(response) = response {
                        match response {
                            AudioResponse::OutputStreams(s) => s,
                            _ => Vec::new(),
                        }
                    } else {
                        Vec::new()
                    };
                    ctx.reply(Ok((output_streams,)))
                }
            },
        );
        c.method_with_cr_async(
            "SetSourceOfOutputStream",
            ("input_stream", "source"),
            (),
            move |mut ctx, cross, (output_stream, source): (u32, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ =
                        sender.send(AudioRequest::SetSourceOfOutputStream(output_stream, source));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "SetOutputStreamVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ =
                        sender.send(AudioRequest::SetOutputStreamVolume(index, channels, volume));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async(
            "SetOutputStreamMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetOutputStreamMute(index, muted));
                    ctx.reply(Ok(()))
                }
            },
        );
        c.method_with_cr_async("ListCards", (), ("cards",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let sender = data.audio_sender.clone();
            let receiver = data.audio_receiver.clone();
            async move {
                let _ = sender.send(AudioRequest::ListCards);
                let response = receiver.recv();
                let cards: Vec<Card> = if let Ok(response) = response {
                    match response {
                        AudioResponse::Cards(s) => s,
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                ctx.reply(Ok((cards,)))
            }
        });
        c.method_with_cr_async(
            "SetCardProfileOfDevice",
            ("device_index", "profile_name"),
            (),
            move |mut ctx, cross, (device_index, profile_name): (u32, String)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let sender = data.audio_sender.clone();
                async move {
                    let _ = sender.send(AudioRequest::SetCardProfileOfDevice(
                        device_index,
                        profile_name,
                    ));
                    ctx.reply(Ok(()))
                }
            },
        );
    });
    token
}
