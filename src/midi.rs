use std::{
    sync::Arc,
    thread::{self},
};

use crossbeam::channel::Receiver;
use midir::MidiOutput;
use parking_lot::Mutex;
use crate::{
    context::{AppState, Context},
    note_events::Note,
};

pub const _NOTE_ON_MESSAGE: u8 = 0x90;
pub const NOTE_OFF_MESSAGE: u8 = 0x80;
pub const MIDI_CHANNEL_COUNT: u8 = 16;
pub const MIDI_NOTE_COUNT: u8 = 128;


pub fn run_midi(
    midi_note_receiver: Receiver<Vec<Note>>,
    midi_port_receiver: Receiver<usize>,
    midi_context_arc: Arc<Mutex<Context>>,
) {
    thread::spawn(move || {

        // prepare MIDI
        let mut midi_out = MidiOutput::new("rust-orca").unwrap();
        let out_ports = midi_out.ports();
        let mut default_midi_port = 0;
        let out_port = out_ports
            .get(default_midi_port)
            .ok_or("No MIDI output ports available")
            .unwrap();

        // get and set the name of the default midi port
        let midi_port_name = midi_out.port_name(out_port).unwrap();
        {
            let mut context = midi_context_arc.lock();
            context.midi_port_name = midi_port_name.clone();
        }

        // connect to the default midi port
        let mut midi_conn = midi_out.connect(out_port, "rust-orca-conn").unwrap();

        // clear all existing midi notes on start
        for channel in 0..MIDI_CHANNEL_COUNT {
            for note in 0..MIDI_NOTE_COUNT {
                let note_off_message = NOTE_OFF_MESSAGE + channel;
                midi_conn.send(&[note_off_message, note, 0]).unwrap();
            }
        }

        // run the main loop
        loop {
            // set the new midi port if changed
            let requested_midi_port = midi_port_receiver.recv().unwrap();
            if requested_midi_port != default_midi_port {
                default_midi_port = requested_midi_port;
                midi_out = midi_conn.close();
                let out_ports = midi_out.ports();
                let out_port = out_ports.get(requested_midi_port % out_ports.len())
                    .ok_or("No MIDI output ports available")
                    .unwrap();
                let midi_port_name = midi_out.port_name(out_port).unwrap();
                let mut context = midi_context_arc.lock();
                context.midi_port_name = midi_port_name.clone();
                midi_conn = midi_out.connect(out_port, "rust-orca-conn").unwrap();
            }

            // process notes
            let mut notes = midi_note_receiver.recv().unwrap();
            for note in notes.iter_mut() {
                if note.started && note.duration == 0 {
                    note.stop(&mut midi_conn);
                } else if !note.started {
                    note.stop(&mut midi_conn);
                    note.start(&mut midi_conn);
                }
            }

            // clear all midi notes on shutdown
            let is_shutdown = { midi_context_arc.lock().app_state };
            if is_shutdown == AppState::Shutdown {
                for channel in 0..MIDI_CHANNEL_COUNT {
                    for note in 0..MIDI_NOTE_COUNT {
                        let note_off_message = NOTE_OFF_MESSAGE + channel;
                        midi_conn.send(&[note_off_message, note, 0]).unwrap();
                    }
                }
            }
        }
    });
}

pub fn run_midi_cc(midi_cc_receiver: Receiver<Vec<Note>>) {
    let midi_out = MidiOutput::new("rust-orca").unwrap();
    let out_ports = midi_out.ports();
    let out_port = out_ports
        .get(0)
        .ok_or("No MIDI output ports available")
        .unwrap();
    let mut conn = midi_out.connect(out_port, "rust-orca-conn").unwrap();

    thread::spawn(move || {
        loop {
            // process notes
            let mut notes = midi_cc_receiver.recv().unwrap();
            for note in notes.iter_mut() {
                if note.started && note.duration == 0 {
                    note.stop(&mut conn);
                } else if !note.started {
                    note.stop(&mut conn);
                    note.start(&mut conn);
                    conn.send(&[
                        note.channel,
                        note.degree,
                        scale_exponential(note.velocity as f32),
                    ])
                        .unwrap();
                }
            }
        }
    });
}


// scale velocity
fn scale_exponential(input: f32) -> u8 {
    let old_min = 0.0;
    let old_max = 36.0;
    let new_min = 0.0;
    let new_max = 127.0;

    // scale input to 0-1
    let normalized = (input - old_min) / (old_max - old_min);

    // apply exponential function
    let exp = 2.0_f32.powf(normalized);

    // scale output to 0-127
    (exp * (new_max - new_min) + new_min) as u8
}
