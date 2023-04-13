use std::sync::mpsc::{Sender, Receiver};
use mpvipc::{Error, Event, Mpv, MpvDataType, Property};

use crate::app::Command;

#[derive(Debug, Clone)]
pub struct MpvState {
    pub running: bool,
    pub pause: bool,
    pub path: Option<String>,
    pub playback_time: f64,
    pub duration: f64
}
impl MpvState {
    pub fn new() -> Self {
        MpvState {
            running: false,
            pause: false,
            path: None,
            playback_time: 0.0,
            duration: 0.0
        }
    }
}

pub struct MpvMsg {
    pub state: MpvState,
}
impl MpvMsg {
    pub fn new() -> Self {
        MpvMsg {
            state: MpvState::new()
        }
    }
    pub fn for_state(state: MpvState) -> Self {
        MpvMsg {
            state
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

pub fn mpv_stop(state: &mut MpvState) {
    let mut running = true;
    let mut stopping = false;
    println!("Stopping mpv.");
    while running {
        let mut mpv = Mpv::connect("/tmp/mpv.sock");
        match &mut mpv {
            Ok(m) => {
                if !stopping {
                    match m.stop() {
                        _ => () //Just ignore
                    }
                    stopping = true
                }
            },
            _ => { running = false; println!("Stopped.") },
        }
    }
    state.running = false;
}


pub fn watch_mpv(state: &mut MpvState, sender: &mut Sender<MpvMsg>) {
    if !state.running {
        state.running = true;
        let sender = sender.clone();
        std::thread::spawn(move || {
            loop { 
                //println!("Connecting to mpv.");
                let mut mpv = Mpv::connect("/tmp/mpv.sock");
                match &mut mpv {
                    Ok(m) => {
                        //println!("Connected!");
                        let mut new_state = get_mpv_state(m);
                        //println!("Initial path:{}", new_state.clone().path.unwrap_or("".to_string()));
                        sender.send(MpvMsg::for_state(new_state.clone())).unwrap();
                        observe(m);
                        loop {
                            let event = m.event_listen();
                            match event {
                                Ok(e) => {
                                    match e {
                                        Event::PropertyChange { id: _, property } => {
                                            match property {
                                                Property::Pause(value) => { new_state.pause = value;  /*  println!("Pause") */ },
                                                Property::Path(Some(value)) => { new_state.path = Some(value.clone()); /*  println!("Set path:{}", value) */ },
                                                Property::PlaybackTime(Some(value)) => new_state.playback_time = value,
                                                Property::PlaybackTime(None) => new_state.playback_time = 0.0,
                                                Property::Duration(Some(value)) => new_state.duration = value,
                                                Property::Duration(None) => new_state.duration = 0.0,
                                                _ => (),
                                            }
                                            let _ = sender.send(MpvMsg::for_state(new_state.clone()));
                                        },
                                        Event::Shutdown => (),
                                        Event::EndFile => (),
                                        _ => {
                                            observe(m);
                                        },
                                    }
                                },
                                _ =>  {

                                    println!("Failed to get event!");
                                    let _ = sender.send(MpvMsg::new()); return },
                            }
                        }
                    },
                    _ =>  { 
                            println!("Connection failed!");
                            let _ = sender.send(MpvMsg::new()); 
                    },
                }
            }
        });
    }
}

pub fn observe(m: &mut Mpv) {
    m.observe_property(1, "path").unwrap();
    m.observe_property(2, "pause").unwrap();
    m.observe_property(3, "playback-time").unwrap();
    m.observe_property(4, "duration").unwrap();
    m.observe_property(5, "metadata").unwrap();
}

pub fn update_mpv_state(state: &mut MpvState, receiver: &mut Receiver<MpvMsg>) {
    while let Ok(msg) = receiver.try_recv() {
        let new_state = msg.state;
        state.pause = new_state.pause;
        state.running = new_state.running;
        if state.path != new_state.path {
            state.path = new_state.path;
            state.running = true;
        }
        if state.playback_time != new_state.playback_time {
            state.playback_time = new_state.playback_time;
            state.running = true;
            state.pause = false;
        }
        state.duration = new_state.duration;
        //println!("{} - {} - {} - {}", state.running, state.pause, state.playback_time, state.path.as_ref().unwrap_or(&"none".to_string()));
    }
}

fn get_mpv_state(mpv: &mut Mpv) -> MpvState {
    return MpvState { running: true, pause: mpv.get_property("pause").unwrap_or(false), path: mpv.get_property("path").ok(), playback_time: mpv.get_property("playback_time").unwrap_or(0.0), duration: mpv.get_property("duration").unwrap_or(0.0) };
}
