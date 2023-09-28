use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
    thread::{self, sleep},
    time::{Duration, Instant},
};
use thread::spawn;

use crossbeam::channel::Sender;
use midir::MidiOutputConnection;
use parking_lot::Mutex;

use crate::{context::{Context, AppState}, NoteSenders, operators::get_tick_operators,
            operators::get_bang_operators,
            operators::grid_tick,
            operators::read_operator_config,
            utils::{NATURAL_NOTES, SHARP_NOTES}};

const NOTE_ON_MESSAGE: u8 = 0x90;
const NOTE_OFF_MESSAGE: u8 = 0x80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiCC {
    pub channel: u8,
    pub command: u8,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    pub note_type: u8,
    pub channel: u8,
    pub engine: u8,
    pub sample: u8,
    pub slot: u8,
    pub note_number: u8,
    pub velocity: u8,
    pub duration: u64,
    pub reverb: u8,
    pub started: bool,
    pub degree: u8,
    pub speed: u8,
}

impl Note {
    pub fn from_base_36(
        note_type: u8,
        channel: u8,
        engine: u8,
        sample: u8,
        slot: u8,
        base_octave: u8,
        base_note: u8,
        sharp: bool,
        degree: u8,
        velocity: u8,
        duration: u8,
        reverb: u8,
        tick_time: u64,
        speed: u8,
    ) -> Note {
        let note_index = (base_note - 10) % 7;
        let octave_offset = 1 + (base_note - 10) / 7;
        let note_index = note_index as usize;

        let note_offset = match sharp {
            true => SHARP_NOTES[note_index],
            false => NATURAL_NOTES[note_index],
        };

        let octave = base_octave + octave_offset;
        let note_number = 12 * octave + note_offset;
        let velocity = (velocity as f32 * (127.0 / 35.0)) as u8;
        let duration = duration as u64 * tick_time;

        Note {
            note_type,
            channel,
            engine,
            sample,
            slot,
            note_number,
            velocity,
            duration,
            started: false,
            degree,
            reverb,
            speed,
        }
    }

    pub fn start(&mut self, conn: &mut MidiOutputConnection) {
        let note_on_message: u8 = NOTE_ON_MESSAGE + self.channel;
        if let Err(err) = conn.send(&[note_on_message, self.note_number, self.velocity]) {
            println!("Midi note on send error: {}", err);
        } else {
            self.started = true;
        };
    }

    pub fn stop(&self, conn: &mut MidiOutputConnection) {
        let note_off_message: u8 = NOTE_OFF_MESSAGE + self.channel;
        if let Err(err) = conn.send(&[note_off_message, self.note_number, self.velocity]) {
            println!("Midi note off send error: {}", err);
        }
    }
}

pub fn notes_tick(notes: &[Note], tick_time: u64) -> Vec<Note> {
    let mut note_set: HashMap<(u8, u8), Note> = HashMap::new();
    for note in notes {
        let key = (note.channel, note.note_number);
        if note.started {
            let duration = note.duration.saturating_sub(tick_time);
            if let Some(other_note) = note_set.get(&key) {
                if other_note.duration >= duration {
                    continue;
                }
            }
            let mut note = *note;
            note.duration = duration;
            note_set.insert(key, note);
        } else {
            note_set.insert(key, *note);
        }
    }
    note_set.values().cloned().collect()
}

fn process_and_send_notes(
    midi_notes: &[Note],
    tick_time: f64,
    midi_port: usize,
    note_senders: &NoteSenders,
    midi_port_sender: &Sender<usize>
) -> Vec<Note> {
    let mut processed_notes = notes_tick(
        midi_notes,
        tick_time as u64
    );
    let mut midi_notes_to_play = Vec::new();
    let mut midi_cc_to_play = Vec::new();
    let mut sampler_notes_to_play = Vec::new();
    let mut synth_notes_to_play = Vec::new();
    for note in processed_notes.iter_mut() {
        match note.note_type {
            0 => {
                midi_notes_to_play.push(*note);
                let _ = note_senders.midi_note_sender.send(midi_notes_to_play.clone());
                note.started = true;
                midi_port_sender.send(midi_port).unwrap();
            }
            1 => if !note.started {
                synth_notes_to_play.push(*note);
                let _ = note_senders.synth_note_sender.send(synth_notes_to_play.clone());
                note.started = true;
            },
            2 => if !note.started {
                sampler_notes_to_play.push(*note);
                let _ = note_senders.sampler_note_sender.send(sampler_notes_to_play.clone());
                note.started = true;
            },
            3 => {
                midi_cc_to_play.push(*note);
                let _ = note_senders.midi_cc_sender.send(midi_cc_to_play.clone());
                note.started = true;
            }
            _ => println!("bam"),
        }
    }
    processed_notes.iter().filter(|note| note.duration > 0).cloned().collect()
}

pub fn run_notes(
    notes_context_arc: Arc<Mutex<Context>>,
    should_redraw_notes: Arc<AtomicBool>,
    note_senders: NoteSenders,
    midi_port_sender: Sender<usize>,
) {
    let operator_map = read_operator_config("operator_config.txt");
    let tick_operators = get_tick_operators(&operator_map);
    let bang_operators = get_bang_operators(&operator_map);
    spawn(move || {
        let mut next_tick = Instant::now();
        loop {
            let now = Instant::now();
            if now >= next_tick {
                // Get and lock app state
                let mut context_locked = notes_context_arc.lock();

                if context_locked.app_state == AppState::Running {
                    grid_tick(
                        &mut context_locked,
                        &tick_operators,
                        &bang_operators,
                        should_redraw_notes.clone(),
                    );


                    let midi_notes = context_locked.notes.clone();
                    let tick_time = context_locked.tick_time;
                    let midi_port = context_locked.midi_port;
                    context_locked.notes = process_and_send_notes(
                        &midi_notes,
                        tick_time as f64,
                        midi_port as usize,
                        &note_senders,
                        &midi_port_sender
                    );

                    let tick_duration = Duration::from_secs_f64(60.0 / (context_locked.divisions * context_locked.tempo) as f64);
                    next_tick += tick_duration;
                }
                drop(context_locked);
            } else {
                sleep(next_tick - now);
            }
        }
    });
}

