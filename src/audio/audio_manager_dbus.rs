use std::{
    rc::Rc,
    sync::{
        atomic::Ordering,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use dbus_crossroads::Crossroads;
use re_set_lib::audio::audio_structures::{Card, InputStream, OutputStream, Sink, Source};

use crate::{
    utils::{AudioRequest, AudioResponse},
    DaemonData,
};

use super::audio_manager::PulseServer;

pub fn setup_audio_manager(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<DaemonData> {
    // TODO handle errors on the now not bool returning functions
    let token = cross.register("org.Xetibo.ReSetAudio", |c| {
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
        c.method_with_cr_async("StartAudioListener", (), (), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            if !data.audio_listener_active.load(Ordering::SeqCst) {
                let (dbus_pulse_sender, pulse_receiver): (
                    Sender<AudioRequest>,
                    Receiver<AudioRequest>,
                ) = mpsc::channel();
                let (pulse_sender, dbus_pulse_receiver): (
                    Sender<AudioResponse>,
                    Receiver<AudioResponse>,
                ) = mpsc::channel();

                data.audio_sender = Rc::new(dbus_pulse_sender);
                data.audio_receiver = Rc::new(dbus_pulse_receiver);
                let listener_active = data.audio_listener_active.clone();
                let connection = data.connection.clone();
                thread::spawn(move || {
                    let res = PulseServer::create(pulse_sender, pulse_receiver, connection);
                    if res.is_err() {
                        return;
                    }
                    listener_active.store(true, Ordering::SeqCst);
                    res.unwrap().listen_to_messages();
                });
            }
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async("StopAudioListener", (), (), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            if data.audio_listener_active.load(Ordering::SeqCst) {
                let _ = data.audio_sender.send(AudioRequest::StopListener);
            }
            data.audio_listener_active.store(false, Ordering::SeqCst);
            async move { ctx.reply(Ok(())) }
        });
        c.method_with_cr_async(
            "GetDefaultSink",
            (),
            ("default_sink",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::GetDefaultSink);
                let response = data.audio_receiver.recv();
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
                async move { ctx.reply(response) }
            },
        );
        c.method_with_cr_async(
            "GetDefaultSinkName",
            (),
            ("sink_name",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::GetDefaultSinkName);
                let response = data.audio_receiver.recv();
                let sink_name = if let Ok(response) = response {
                    match response {
                        AudioResponse::DefaultSinkName(s) => s,
                        _ => String::from(""),
                    }
                } else {
                    String::from("")
                };
                async move { ctx.reply(Ok((sink_name,))) }
            },
        );
        c.method_with_cr_async(
            "GetDefaultSource",
            (),
            ("default_source",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::GetDefaultSource);
                let response = data.audio_receiver.recv();
                let source: Option<Source> = if let Ok(response) = response {
                    match response {
                        AudioResponse::DefaultSource(s) => Some(s),
                        _ => None,
                    }
                } else {
                    None
                };
                let response: Result<(Source,), dbus::MethodErr> = if let Some(source) = source {
                    Ok((source,))
                } else {
                    Err(dbus::MethodErr::failed("Could not get default source"))
                };
                async move { ctx.reply(response) }
            },
        );
        c.method_with_cr_async(
            "GetDefaultSourceName",
            (),
            ("source_name",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::GetDefaultSourceName);
                let response = data.audio_receiver.recv();
                let source_name = if let Ok(response) = response {
                    match response {
                        AudioResponse::DefaultSourceName(s) => s,
                        _ => String::from(""),
                    }
                } else {
                    String::from("")
                };
                async move { ctx.reply(Ok((source_name,))) }
            },
        );
        c.method_with_cr_async("ListSinks", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let _ = data.audio_sender.send(AudioRequest::ListSinks);
            let response = data.audio_receiver.recv();
            let sinks: Vec<Sink> = if let Ok(response) = response {
                match response {
                    AudioResponse::Sinks(s) => s,
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            async move { ctx.reply(Ok((sinks,))) }
        });
        c.method_with_cr_async("ListSources", (), ("sinks",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let _ = data.audio_sender.send(AudioRequest::ListSources);
            let response = data.audio_receiver.recv();
            let sources: Vec<Source> = if let Ok(response) = response {
                match response {
                    AudioResponse::Sources(s) => s,
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            async move { ctx.reply(Ok((sources,))) }
        });
        c.method_with_cr_async(
            "SetSinkVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSinkVolume(index, channels, volume));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetSinkMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSinkMute(index, muted));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetSourceVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSourceVolume(index, channels, volume));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetSourceMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSourceMute(index, muted));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetDefaultSink",
            ("sink",),
            (),
            move |mut ctx, cross, (sink,): (String,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::SetDefaultSink(sink));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetDefaultSource",
            ("source",),
            (),
            move |mut ctx, cross, (source,): (String,)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetDefaultSource(source));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ListInputStreams",
            (),
            ("input_streams",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::ListInputStreams);
                let response = data.audio_receiver.recv();
                let input_streams: Vec<InputStream> = if let Ok(response) = response {
                    match response {
                        AudioResponse::InputStreams(s) => s,
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };
                async move { ctx.reply(Ok((input_streams,))) }
            },
        );
        c.method_with_cr_async(
            "SetSinkOfInputStream",
            ("input_stream", "sink"),
            (),
            move |mut ctx, cross, (input_stream, sink): (u32, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSinkOfInputStream(input_stream, sink));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetInputStreamVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetInputStreamVolume(index, channels, volume));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetInputStreamMute",
            ("input_stream_index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetInputStreamMute(index, muted));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "ListOutputStreams",
            (),
            ("output_streams",),
            move |mut ctx, cross, ()| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data.audio_sender.send(AudioRequest::ListOutputStreams);
                let response = data.audio_receiver.recv();
                async move {
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
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetSourceOfOutputStream(output_stream, source));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetOutputStreamVolume",
            ("index", "channels", "volume"),
            (),
            move |mut ctx, cross, (index, channels, volume): (u32, u16, u32)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetOutputStreamVolume(index, channels, volume));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async(
            "SetOutputStreamMute",
            ("index", "muted"),
            (),
            move |mut ctx, cross, (index, muted): (u32, bool)| {
                let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
                let _ = data
                    .audio_sender
                    .send(AudioRequest::SetOutputStreamMute(index, muted));
                async move { ctx.reply(Ok(())) }
            },
        );
        c.method_with_cr_async("ListCards", (), ("cards",), move |mut ctx, cross, ()| {
            let data: &mut DaemonData = cross.data_mut(ctx.path()).unwrap();
            let _ = data.audio_sender.send(AudioRequest::ListCards);
            let response = data.audio_receiver.recv();
            async move {
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
                let _ = data.audio_sender.send(AudioRequest::SetCardProfileOfDevice(
                    device_index,
                    profile_name,
                ));
                async move { ctx.reply(Ok(())) }
            },
        );
    });
    token
}
