use std::sync::mpsc::{Sender, Receiver};
use mpvipc::{Error, Event, Mpv, MpvDataType, Property};

use crate::app::Command;

#[derive(Debug, Clone)]
pub struct MpvState {
    pub listening: bool,
    pub pause: bool,
    pub path: Option<String>,
    pub playback_time: f64,
    pub duration: f64
}
impl MpvState {
    pub fn new() -> Self {
        MpvState {
            listening: false,
            pause: false,
            path: None,
            playback_time: 0.0,
            duration: 0.0
        }
    }
}

pub fn mpv_pause() {
    let mut mpv = Mpv::connect("/tmp/mpv.sock");
    match &mut mpv {
        Ok(m) => {
            let paused: bool = m.get_property("pause").unwrap();
            m.set_property("pause", !paused).expect("Error pausing");
        },
        _ => ()
    }
}

pub fn mpv_play(path: String) {
    let mut mpv = Mpv::connect("/tmp/mpv.sock");
    match &mut mpv {
        Ok(m) => {
            m.set_property("path", path).expect("Error playing");
        },
        _ => ()
    }
}

pub fn watch_mpv(state: &mut MpvState, sender: &mut Sender<Event>) {
    if !state.listening {
        state.listening = true;
        let sender = sender.clone();
        std::thread::spawn(move || {
            loop {
                println!("Connecting to tmp.sock ...");
                let mut mpv = Mpv::connect("/tmp/mpv.sock");
                match &mut mpv {
                    Ok(m) => {
                        m.observe_property(1, "path").unwrap();
                        m.observe_property(2, "pause").unwrap();
                        m.observe_property(3, "playback-time").unwrap();
                        m.observe_property(4, "duration").unwrap();
                        m.observe_property(5, "metadata").unwrap();
                        loop {
                            let event = m.event_listen().unwrap();
                            sender.send(event).unwrap();
                        }
                    },
                    _ => (),
                }
            }
        });
    }
}

pub fn update_mpv_state(state: &mut MpvState, receiver: &mut Receiver<Event>) {
    while let Ok(event) = receiver.try_recv() {
        match event {
            Event::PropertyChange { id: _, property } => match property {
                Property::Pause(value) => state.pause = value,
                Property::Path(Some(value)) => state.path = Some(value),
                Property::Path(None) => state.path = None,

                Property::PlaybackTime(Some(value)) => state.playback_time = value,
                Property::PlaybackTime(None) => state.playback_time = 0.0,

                Property::Duration(Some(value)) => state.duration = value,
                Property::Duration(None) => state.duration = 0.0,
                _ => (),
            },
            _ => (),
        }
    }
}
