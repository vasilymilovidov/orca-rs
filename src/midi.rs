use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use midir::MidiOutputConnection;

// c c# d d# e e# f f# g g# a a# b  b# c
// 0 1  2 3  4 5  5 6  7 8  9 10 11 12 12
const NATURAL_NOTES: [u8; 7] = [9, 11, 0, 2, 4, 5, 7];
const SHARP_NOTES: [u8; 7] = [10, 12, 1, 3, 5, 6, 8];

#[derive(Debug)]
#[derive(Clone)]
#[derive(Copy)]
pub struct MidiNote {
    pub channel: u8,
    pub note_number: u8,
    // TODO split into attack/release velocity
    pub velocity: u8,
    pub duration: u64,
    pub started: bool,
}

impl MidiNote {
    pub fn from_base_36(channel: u8, base_octave: u8, base_note: u8, sharp: bool, velocity: u8,
                        duration: u8, tick_time: u64) -> MidiNote {
        let note_index = (base_note - 10) % 7;
        let octave_offset = 1 + (base_note - 10) / 7;
        let note_index = note_index as usize;
        let note_offset = if sharp { SHARP_NOTES[note_index] } else { NATURAL_NOTES[note_index] };
        let octave = base_octave + octave_offset;
        let note_number = 12 * octave + note_offset;

        let velocity = (velocity as f32 * (127.0 / 35.0)) as u8;

        let duration = duration as u64 * tick_time;
        MidiNote { channel, note_number, velocity, duration, started: false }
    }

    pub fn play(&self, conn: &mut MidiOutputConnection) {
        let note_on_message: u8 = 0x90 + self.channel;
        let note_off_message: u8 = 0x80 + self.channel;
        conn.send(&[note_on_message, self.note_number, self.velocity]);
        sleep(Duration::from_millis(self.duration));
        conn.send(&[note_off_message, self.note_number, self.velocity]);
    }

    pub fn start(&mut self, conn: &mut MidiOutputConnection) {
        let note_on_message: u8 = 0x90 + self.channel;
        conn.send(&[note_on_message, self.note_number, self.velocity]);
        self.started = true;
    }

    pub fn stop(&self, conn: &mut MidiOutputConnection) {
        let note_off_message: u8 = 0x80 + self.channel;
        conn.send(&[note_off_message, self.note_number, self.velocity]);
    }
}

pub fn notes_tick(notes: &Vec<MidiNote>, tick_time: u64) -> Vec<MidiNote> {
    let mut note_set: HashMap<(u8, u8), MidiNote> = HashMap::new();
    for &note in notes {
        if note.started {
            let duration = note.duration.saturating_sub(tick_time);
            let key = (note.channel, note.note_number);
            if let Some(other_note) = note_set.get(&key) {
                if other_note.duration < duration {
                    let mut note = note.clone();
                    note.duration = duration;
                    note_set.insert(key, note);
                }
            } else {
                let mut note = note.clone();
                note.duration = duration;
                note_set.insert(key, note);
            }
        } else {
            let key = (note.channel, note.note_number);
            note_set.insert(key, note);
        }
    }

    note_set.values().cloned().collect()
}
