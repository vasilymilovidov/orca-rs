use copypasta::{ClipboardContext, ClipboardProvider};
use rand::{
    distributions::Bernoulli,
    prelude::Distribution,
    thread_rng,
    Rng
};
use std::{
    collections::HashMap,
    fs::{self, read_to_string, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc
};
use crate::context::{Context, Globals, Port};
use crate::note_events::Note;

use crate::utils::{NATURAL_NOTES, SCALES, SHARP_NOTES};

pub fn char_to_base_36(c: char) -> (u8, bool) {
    match c {
        '0'..='9' => (c as u8 - b'0', false),
        'a'..='z' => (c as u8 - b'a' + 10, false),
        'A'..='Z' => (c as u8 - b'A' + 10, true),
        _ => (0, false),
    }
}

pub fn base_36_to_char(c: u8, upper: bool) -> char {
    let c = c % 36;
    match c {
        0..=9 => (c + b'0') as char,
        10..=35 if upper => (c - 10 + b'A') as char,
        10..=35 => (c - 10 + b'a') as char,
        _ => unreachable!(),
    }
}

pub enum Update {
    Inputs(Vec<Port>),
    Outputs(Vec<Port>),
    Locks(Vec<Port>),
    Notes(Vec<Note>),
    Variables(Vec<(char, char)>),
    Globals(Globals),
    Save(String),
    Load(String),
}

#[derive(Clone)]
pub struct Operator {
    name: String,
    pub evaluate: fn(context: &Context, row: i32, col: i32) -> Vec<Update>,
    input_ports: Vec<String>,
    output_ports: Vec<String>,
}

impl Operator {
    fn new(
        name: &str,
        evaluate: fn(&Context, i32, i32) -> Vec<Update>,
        input_ports: Vec<String>,
        output_ports: Vec<String>,
    ) -> Operator {
        Operator {
            name: String::from(name),
            evaluate,
            input_ports,
            output_ports,
        }
    }

    fn apply(&self, context: &mut Context, row: i32, col: i32) {
        if !context.is_locked(row, col) {
            let updates = (self.evaluate)(context, row, col);
            for update in updates {
                match update {
                    Update::Inputs(ports) => {
                        for (index, port) in ports.iter().enumerate() {
                            context.lock_with_name(
                                port.row,
                                port.col,
                                self.input_ports[index].clone(),
                            );
                        }
                    }
                    Update::Outputs(ports) => {
                        for (index, port) in ports.iter().enumerate() {
                            context.write(port.row, port.col, port.value);
                            context.lock_with_name(
                                port.row,
                                port.col,
                                self.output_ports[index].clone(),
                            );
                        }
                    }
                    Update::Locks(ports) => {
                        for port in ports {
                            context.lock(port.row, port.col);
                        }
                    }
                    Update::Notes(notes) => {
                        for note in notes {
                            context.write_note(note);
                        }
                    }
                    Update::Globals(globals) => {
                        context.global_key = globals.global_key;
                        context.global_scale = globals.global_scale;
                    }
                    Update::Load(name) => {
                        context.load(name);
                    }
                    Update::Save(name) => {
                        context.save(name);
                    }
                    Update::Variables(variables) => {
                        for (name, value) in variables {
                            context.set_variable(name, value);
                        }
                    }
                }
            }
        }
    }
}

pub fn read_operator_config(filename: &str) -> HashMap<String, char> {
    let default_operator_config = "
B Sub
C Clock
D Delay
E East
F If
G Generate
H Halt
I Increment
J Jump
K Concat
L Lesser
M Multiply
N North
O Read
P Push
Q Query
R Random
S South
T Track
U Euclid
V Variable
W West
X Write
Y Jymp
Z Interpolate
# Comment
~ Synth
: Midi
? MidiCC
; Scaler
> Sampler
^ Bernoulli
± Turing
@ Globals
[ Saver
] Loader
{ SnipSave
} SnipLoad
"
        .trim()
        .to_string();
    read_to_string(filename)
        .unwrap_or(default_operator_config)
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter_map(|(symbol, name)| {
            symbol
                .chars()
                .next()
                .map(|symbol| (name.to_string(), symbol))
        })
        .collect()
}

pub fn get_tick_operators(operator_map: &HashMap<String, char>) -> HashMap<char, Operator> {
    vec![
        Operator::new(
            "Globals",
            global,
            vec!["Global Key".to_string(), "Global Scale".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "SnipSave",
            snippet_saver,
            vec![
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
            ],
            vec!["".to_string()],
        ),
        Operator::new(
            "SnipLoad",
            snippet_loader,
            vec![
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
            ],
            vec!["".to_string()],
        ),
        Operator::new(
            "Saver",
            saver,
            vec![
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
            ],
            vec!["".to_string()],
        ),
        Operator::new(
            "Loader",
            loader,
            vec![
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
                "char".to_string(),
            ],
            vec!["".to_string()],
        ),
        Operator::new(
            "Add",
            add,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["A+B".to_string()],
        ),
        Operator::new(
            "Sub",
            sub,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["A-B".to_string()],
        ),
        Operator::new(
            "Clock",
            clock,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Delay",
            delay,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "East",
            east,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string(), "Output".to_string()],
        ),
        Operator::new(
            "If",
            condition,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["A==B".to_string()],
        ),
        Operator::new(
            "Generate",
            generate,
            vec![
                "Offset Y".to_string(),
                "Offset X".to_string(),
                "Port 0".to_string(),
                "Port 1".to_string(),
                "Port 2".to_string(),
                "Port 3".to_string(),
                "Port 4".to_string(),
                "Port 5".to_string(),
                "Port 6".to_string(),
                "Port 7".to_string(),
                "Port 8".to_string(),
                "Port 9".to_string(),
                "Port a".to_string(),
                "Port b".to_string(),
                "Port c".to_string(),
                "Port d".to_string(),
                "Port e".to_string(),
                "Port f".to_string(),
                "Port g".to_string(),
                "Port h".to_string(),
            ],
            vec![
                "Output 0".to_string(),
                "Output 1".to_string(),
                "Output 2".to_string(),
                "Output 3".to_string(),
                "Output 4".to_string(),
                "Output 5".to_string(),
                "Output 6".to_string(),
                "Output 7".to_string(),
                "Output 8".to_string(),
                "Output 9".to_string(),
                "Output a".to_string(),
                "Output b".to_string(),
                "Output c".to_string(),
                "Output d".to_string(),
                "Output e".to_string(),
                "Output f".to_string(),
                "Output g".to_string(),
                "Output h".to_string(),
            ],
        ),
        Operator::new(
            "Halt",
            halt,
            vec!["Input A".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Increment",
            increment,
            vec!["Min".to_string(), "Max".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Jump",
            jump,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Concat",
            concat,
            vec![
                "Input A".to_string(),
                "Input B".to_string(),
                "Input C".to_string(),
                "Input D".to_string(),
            ],
            vec![
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
            ],
        ),
        Operator::new(
            "Lesser",
            lesser,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["<".to_string()],
        ),
        Operator::new(
            "Multiply",
            multiply,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["A*B".to_string()],
        ),
        Operator::new(
            "North",
            north,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string(), "Output".to_string()],
        ),
        Operator::new(
            "Read",
            read,
            vec![
                "Offset X".to_string(),
                "Offset Y".to_string(),
                "Input".to_string(),
            ],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Push",
            push,
            vec!["Step".to_string(), "Steps".to_string(), "Value".to_string()],
            vec![
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
            ],
        ),
        Operator::new(
            "Query",
            query,
            vec![
                "Input A".to_string(),
                "Input B".to_string(),
                "Input C".to_string(),
                "Input D".to_string(),
                "Input E".to_string(),
                "Input F".to_string(),
                "Input G".to_string(),
                "Input H".to_string(),
                "Input I".to_string(),
                "Input J".to_string(),
                "Input K".to_string(),
                "Input L".to_string(),
                "Input M".to_string(),
                "Input N".to_string(),
                "Input O".to_string(),
                "Input P".to_string(),
                "Input Q".to_string(),
                "Input R".to_string(),
                "Input S".to_string(),
                "Input T".to_string(),
                "Input U".to_string(),
                "Input V".to_string(),
                "Input W".to_string(),
                "Input X".to_string(),
                "Input Y".to_string(),
                "Input Z".to_string(),
                "Input 0".to_string(),
                "Input 1".to_string(),
                "Input 2".to_string(),
                "Input 3".to_string(),
                "Input 4".to_string(),
                "Input 5".to_string(),
                "Input 6".to_string(),
                "Input 7".to_string(),
                "Input 8".to_string(),
                "Input 9".to_string(),
                "Input 10".to_string(),
            ],
            vec![
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
                "Output".to_string(),
            ],
        ),
        Operator::new(
            "Random",
            random,
            vec!["Min".to_string(), "Max".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "South",
            south,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string(), "Output".to_string()],
        ),
        Operator::new(
            "Track",
            track,
            vec![
                "Step".to_string(),
                "Steps".to_string(),
                "Input 0".to_string(),
                "Input 1".to_string(),
                "Input 2".to_string(),
                "Input 3".to_string(),
                "Input 4".to_string(),
                "Input 5".to_string(),
                "Input 6".to_string(),
                "Input 7".to_string(),
                "Input 8".to_string(),
                "Input 9".to_string(),
                "Input A".to_string(),
                "Input B".to_string(),
                "Input C".to_string(),
                "Input D".to_string(),
                "Input E".to_string(),
                "Input F".to_string(),
                "Input G".to_string(),
                "Input H".to_string(),
                "Input I".to_string(),
                "Input G".to_string(),
                "Input K".to_string(),
                "Input L".to_string(),
                "Input M".to_string(),
                "Input N".to_string(),
                "Input O".to_string(),
                "Input P".to_string(),
                "Input Q".to_string(),
                "Input R".to_string(),
                "Input S".to_string(),
                "Input T".to_string(),
                "Input U".to_string(),
                "Input V".to_string(),
                "Input W".to_string(),
                "Input X".to_string(),
                "Input Y".to_string(),
                "Input Z".to_string(),
            ],
            vec!["Output Step".to_string()],
        ),
        Operator::new(
            "Euclid",
            euclid,
            vec![
                "Density".to_string(),
                "Length".to_string(),
                "Rotation".to_string(),
            ],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Variable",
            variable,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "West",
            west,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string(), "Output".to_string()],
        ),
        Operator::new(
            "Write",
            write,
            vec![
                "Input A".to_string(),
                "Input B".to_string(),
                "Input C".to_string(),
            ],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Jymp",
            jymp,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Interpolate",
            interpolate,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Comment",
            comment,
            vec!["Input A".to_string(), "Input B".to_string()],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Synth",
            synth,
            vec![
                "Engine".to_string(),
                "Octave".to_string(),
                "Degree".to_string(),
                "Velocity".to_string(),
                "Duration".to_string(),
                "Reverb".to_string(),
                "FM".to_string(),
            ],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Sampler",
            sampler,
            vec![
                "Slot".to_string(),
                "Sample".to_string(),
                "Velocity".to_string(),
                "Duration".to_string(),
                "Reverb".to_string(),
                "Speed".to_string(),
            ],
            vec!["Output".to_string()],
        ),
        // the midi operator is technically operated each tick, but only produces a note on a bang
        Operator::new(
            "Midi",
            midi_note,
            vec![
                "Channel".to_string(),
                "Octave".to_string(),
                "Base Note".to_string(),
                "Velocity".to_string(),
                "Duration".to_string(),
            ],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "MidiCC",
            midi_cc,
            vec![
                "Channel".to_string(),
                "Program".to_string(),
                "Value".to_string(),
            ],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Scaler",
            scaler,
            vec![
                "Channel".to_string(),
                "Octave".to_string(),
                "Degree".to_string(),
                "Velocity".to_string(),
                "Duration".to_string(),
            ],
            vec!["Output".to_string()],
        ),
        Operator::new(
            "Bernoulli",
            bernoulli,
            vec!["Probability".to_string()],
            vec!["Output A".to_string(), "Output B".to_string()],
        ),
    ]
        .iter()
        .cloned()
        .filter_map(|operator| {
            if let Some(&symbol) = operator_map.get(&operator.name) {
                Some((symbol, operator))
            } else {
                None
            }
        })
        .collect()
}

fn global(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let key_port = context.listen("key", row, col + 1, 'C');
    let scale_port = context.listen("scale", row, col + 2, '0');

    let key = key_port.value;
    let scale = scale_port.value;

    vec![
        Update::Inputs(vec![key_port, scale_port]),
        Update::Globals(Globals {
            global_key: key,
            global_scale: scale,
        }),
    ]
}

fn add(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let a_port = context.listen("a", row, col - 1, '0');
    let b_port = context.listen("b", row, col + 1, '0');

    let (a, a_upper) = char_to_base_36(a_port.value);
    let (b, b_upper) = char_to_base_36(b_port.value);
    let out = base_36_to_char(a + b, a_upper || b_upper);

    let out_port = Port::new("out", row + 1, col, out);

    vec![
        Update::Inputs(vec![a_port, b_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn sub(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let a_port = context.listen("a", row, col - 1, '0');
    let b_port = context.listen("b", row, col + 1, '0');

    let (a, a_upper) = char_to_base_36(a_port.value);
    let (b, b_upper) = char_to_base_36(b_port.value);
    let diff = if a > b { a - b } else { b - a };
    let out = base_36_to_char(diff, a_upper || b_upper);

    let out_port = Port::new("out", row + 1, col, out);

    vec![
        Update::Inputs(vec![a_port, b_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn delay(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let rate_port = context.listen("rate", row, col - 1, '1');
    let mod_port = context.listen("mod", row, col + 1, '8');

    let (rate, _) = char_to_base_36(rate_port.value);
    let (delay_mod, _) = char_to_base_36(mod_port.value);
    let rate = rate.max(1);
    let delay_mod = delay_mod.max(1);

    let mut out_port = context.listen("out", row + 1, col, '.');
    if context.ticks % (rate as usize * delay_mod as usize) == 0 {
        out_port.value = '*';
    }

    vec![
        Update::Inputs(vec![rate_port, mod_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn random(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let min_port = context.listen("min", row, col - 1, '0');
    let max_port = context.listen("max", row, col + 1, 'z');

    let (min, min_upper) = char_to_base_36(min_port.value);
    let (max, max_upper) = char_to_base_36(max_port.value);
    let max = max.max(min + 1); // wow this looks like trash


    let mut rng = thread_rng();
    let r = rng.gen_range(min..max);
    let out = base_36_to_char(r, min_upper || max_upper);
    let out_port = Port::new("out", row + 1, col, out);

    vec![
        Update::Inputs(vec![min_port, max_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn scaler(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let channel_port = context.listen("channel", row, col + 1, '0');
    let octave_port = context.listen("octave", row, col + 2, '2');
    let degree_port = context.listen("degree", row, col + 3, '0');
    let velocity_port = context.listen("velocity", row, col + 4, 'u');
    let duration_port = context.listen("duration", row, col + 5, '2');
    let (channel, _) = char_to_base_36(channel_port.value);
    let (octave, _) = char_to_base_36(octave_port.value);
    let (note, note_upper) = char_to_base_36(context.global_key);
    let (velocity, _) = char_to_base_36(velocity_port.value);
    let (duration, _) = char_to_base_36(duration_port.value);
    let (degree, _) = char_to_base_36(degree_port.value);
    let (scale, _) = char_to_base_36(context.global_scale);
    let note_index = (note - 10) % 7;
    let octave_offset = 1 + (note - 10) / 7;
    let note_number = prepare_note(octave, note_upper, degree, scale, octave_offset, note_index as usize);
    let velocity = (velocity as f32 * (127.0 / 35.0)) as u8;
    let duration = duration as u64 * context.tick_time;

    let (engine, sample, reverb, speed, slot) = (0, 0, 0, 0, 0);
    let midi_notes = if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        vec![Note {
            note_type: 0,
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
        }]
    } else {
        vec![]
    };

    vec![
        Update::Inputs(vec![
            channel_port,
            octave_port,
            degree_port,
            velocity_port,
            duration_port,
        ]),
        Update::Notes(midi_notes),
    ]
}

fn midi_note(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let channel_port = context.listen("channel", row, col + 1, '0');
    let octave_port = context.listen("octave", row, col + 2, '2');
    let note_port = context.listen("note", row, col + 3, 'C');
    let velocity_port = context.listen("velocity", row, col + 4, 'u');
    let duration_port = context.listen("duration", row, col + 5, '1');
    let note_type = 0;

    let (channel, _) = char_to_base_36(channel_port.value);
    let (octave, _) = char_to_base_36(octave_port.value);
    let (note, note_upper) = char_to_base_36(note_port.value);
    let (velocity, _) = char_to_base_36(velocity_port.value);
    let (duration, _) = char_to_base_36(duration_port.value);

    let midi_notes = if note >= 10
        && (context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*')
    {
        vec![Note::from_base_36(
            note_type,
            channel,
            0,
            0,
            0,
            octave,
            note,
            !note_upper,
            0,
            velocity,
            duration,
            0,
            context.tick_time,
            0,
        )]
    } else {
        vec![]
    };

    vec![
        Update::Inputs(vec![
            channel_port,
            octave_port,
            note_port,
            velocity_port,
            duration_port,
        ]),
        Update::Notes(midi_notes),
    ]
}

fn midi_cc(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let channel_port = context.listen("channel", row, col + 1, '0');
    let command_port = context.listen("comman", row, col + 2, '0');
    let value_port = context.listen("value", row, col + 3, '0');

    let (channel, _) = char_to_base_36(channel_port.value);
    let (command, _) = char_to_base_36(command_port.value);
    let (value, _) = char_to_base_36(value_port.value);

    let channel = channel + 176;

    let midi_cc = if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        vec![Note {
            note_type: 3,
            channel,
            engine: 0,
            sample: 0,
            slot: 0,
            note_number: 0,
            velocity: value,
            duration: 1,
            reverb: 0,
            started: false,
            degree: command,
            speed: 0,
        }]
    } else {
        vec![]
    };

    vec![
        Update::Inputs(vec![channel_port, command_port, value_port]),
        Update::Notes(midi_cc),
    ]
}

fn synth(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let engine_port = context.listen("engine", row, col + 1, '0');
    let octave_port = context.listen("octave", row, col + 2, '2');
    let degree_port = context.listen("degree", row, col + 3, '0');
    let velocity_port = context.listen("velocity", row, col + 4, '9');
    let duration_port = context.listen("duration", row, col + 5, '2');
    let reverb_port = context.listen("reverb", row, col + 6, '0');
    let fm_port = context.listen("fm", row, col + 7, '1');

    let (engine, _) = char_to_base_36(engine_port.value);
    let (octave, _) = char_to_base_36(octave_port.value);
    let (note, note_upper) = char_to_base_36(context.global_key);
    let (velocity, _) = char_to_base_36(velocity_port.value);
    let (duration, _) = char_to_base_36(duration_port.value);
    let (degree, _) = char_to_base_36(degree_port.value);
    let (scale, _) = char_to_base_36(context.global_scale);
    let (reverb, _) = char_to_base_36(reverb_port.value);
    let (fm, _) = char_to_base_36(fm_port.value);
    let note_index = (note - 10) % 7;
    let octave_offset = 1 + (note - 10) / 7;
    let note_number = prepare_note(octave, note_upper, degree, scale, octave_offset, note_index as usize);
    let velocity = (velocity as f32 * (127.0 / 35.0)) as u8;
    let duration = duration as u64 * context.tick_time;

    let midi_notes = if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        vec![Note {
            note_type: 1,
            channel: 0,
            engine,
            sample: 0,
            slot: 0,
            note_number,
            velocity,
            duration,
            started: false,
            degree,
            reverb,
            speed: fm,
        }]
    } else {
        vec![]
    };

    vec![
        Update::Inputs(vec![
            engine_port,
            octave_port,
            degree_port,
            velocity_port,
            duration_port,
            reverb_port,
            fm_port,
        ]),
        Update::Notes(midi_notes),
    ]
}

fn prepare_note(octave: u8, note_upper: bool, degree: u8, scale: u8, octave_offset: u8, note_index: usize) -> u8 {
    let note_offset = if !note_upper { SHARP_NOTES[note_index] } else { NATURAL_NOTES[note_index] };
    let octave = octave + octave_offset;
    let selected_scale = SCALES.get(scale as usize % 26).expect("invalid scale");
    let scale_offset = match degree {
        0..=6 => 0,
        7..=13 => 12,
        14..=20 => 24,
        21..=27 => 36,
        28..=34 => 48,
        _ => 60,
    } + *selected_scale.get((degree % 7) as usize).expect("invalid degree");
    let note_number = scale_offset + 12 * octave + note_offset;
    note_number
}

fn sampler(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let slot_port = context.listen("slot", row, col + 1, '0');
    let sample_port = context.listen("sample", row, col + 2, '0');
    let velocity_port = context.listen("velocity", row, col + 3, '9');
    let duration_port = context.listen("duration", row, col + 4, '4');
    let reverb_port = context.listen("reverb", row, col + 5, '0');
    let speed_port = context.listen("reverb", row, col + 6, '1');

    let (slot, _) = char_to_base_36(slot_port.value);
    let (sample, _) = char_to_base_36(sample_port.value);
    let (velocity, _) = char_to_base_36(velocity_port.value);
    let (duration, _) = char_to_base_36(duration_port.value);
    let (reverb, _) = char_to_base_36(reverb_port.value);
    let (speed, _) = char_to_base_36(speed_port.value);

    let sampler_notes = if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        vec![Note::from_base_36(
            2,
            0,
            0,
            sample,
            slot % 4,
            0,
            slot,
            false,
            0,
            velocity,
            duration,
            reverb,
            context.tick_time,
            speed,
        )]
    } else {
        vec![]
    };

    vec![
        Update::Inputs(vec![
            slot_port,
            sample_port,
            velocity_port,
            duration_port,
            reverb_port,
            speed_port,
        ]),
        Update::Notes(sampler_notes),
    ]
}

fn clock(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let rate_port = context.listen("rate", row, col - 1, '1');
    let mod_port = context.listen("mod", row, col + 1, '8');

    let (rate, _) = char_to_base_36(rate_port.value);
    let (clock_mod, mod_upper) = char_to_base_36(mod_port.value);
    let rate = rate.max(1);
    let clock_mod = clock_mod.max(1);
    let out = context.ticks / rate as usize % clock_mod as usize;
    let out = base_36_to_char(out as u8, mod_upper);

    let out_port = Port::new("out", row + 1, col, out);

    vec![
        Update::Inputs(vec![rate_port, mod_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn track(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let key_port = context.listen("key", row, col - 2, '0');
    let len_port = context.listen("len", row, col - 1, '1');

    let (key, _) = char_to_base_36(key_port.value);
    let (len, _) = char_to_base_36(len_port.value);
    let len = len.max(1);
    let val_port = context.listen("val", row, col + 1 + (key % len) as i32, '\0');
    let out = val_port.value;

    let out_port = Port::new("out", row + 1, col, out);
    let locks = (0..(len as i32))
        .map(|i| Port::new("locked", row, col + 1 + i, '\0'))
        .collect();

    vec![
        Update::Inputs(vec![key_port, len_port, val_port]),
        Update::Outputs(vec![out_port]),
        Update::Locks(locks),
    ]
}

fn halt(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let output_port = context.listen("out", row + 1, col, '\0');
    vec![
        Update::Inputs(vec![output_port.clone()]),
        Update::Outputs(vec![output_port.clone()]),
        Update::Locks(vec![output_port]),
    ]
}

fn east(context: &Context, row: i32, col: i32) -> Vec<Update> {
    if col + 1 >= context.cols as i32 {
        let mut input_port = context.listen("", row, col, '.');
        input_port.value = '*';
        return vec![Update::Outputs(vec![input_port])];
    }

    let mut input_port = context.listen("", row, col, '.');
    let mut output_port = context.listen("", row, col + 1, '.');

    if output_port.value == '.' {
        output_port.value = input_port.value;
        input_port.value = '.';
        vec![
            Update::Outputs(vec![input_port, output_port.clone()]),
            Update::Locks(vec![output_port]),
        ]
    } else {
        input_port.value = '*';
        vec![Update::Outputs(vec![input_port])]
    }
}

fn west(context: &Context, row: i32, col: i32) -> Vec<Update> {
    if col - 1 < 0 {
        let mut input_port = context.listen("", row, col, '.');
        input_port.value = '*';
        return vec![Update::Outputs(vec![input_port])];
    }

    let mut input_port = context.listen("", row, col, '.');
    let mut output_port = context.listen("", row, col - 1, '.');

    if output_port.value == '.' {
        output_port.value = input_port.value;
        input_port.value = '.';
        vec![
            Update::Outputs(vec![input_port, output_port.clone()]),
            Update::Locks(vec![output_port]),
        ]
    } else {
        input_port.value = '*';
        vec![Update::Outputs(vec![input_port])]
    }
}

fn north(context: &Context, row: i32, col: i32) -> Vec<Update> {
    if row - 1 < 0 {
        let mut input_port = context.listen("", row, col, '.');
        input_port.value = '*';
        return vec![Update::Outputs(vec![input_port])];
    }

    let mut input_port = context.listen("", row, col, '.');
    let mut output_port = context.listen("", row - 1, col, '.');

    if output_port.value == '.' {
        output_port.value = input_port.value;
        input_port.value = '.';
        vec![
            Update::Outputs(vec![input_port, output_port.clone()]),
            Update::Locks(vec![output_port]),
        ]
    } else {
        input_port.value = '*';
        vec![Update::Outputs(vec![input_port])]
    }
}

fn south(context: &Context, row: i32, col: i32) -> Vec<Update> {
    if row + 1 >= context.rows as i32 {
        let mut input_port = context.listen("", row, col, '.');
        input_port.value = '*';
        return vec![Update::Outputs(vec![input_port])];
    }

    let mut input_port = context.listen("", row, col, '.');
    let mut output_port = context.listen("", row + 1, col, '.');

    if output_port.value == '.' {
        output_port.value = input_port.value;
        input_port.value = '.';
        vec![
            Update::Outputs(vec![input_port, output_port.clone()]),
            Update::Locks(vec![output_port]),
        ]
    } else {
        input_port.value = '*';
        vec![Update::Outputs(vec![input_port])]
    }
}

fn condition(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let a_port = context.listen("a", row, col - 1, '\0');
    let b_port = context.listen("b", row, col + 1, '\0');

    let (a, _) = char_to_base_36(a_port.value);
    let (b, _) = char_to_base_36(b_port.value);
    let mut out_port = context.listen("out", row + 1, col, '\0');
    if a == b {
        out_port.value = '*';
    }

    vec![
        Update::Inputs(vec![a_port, b_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn increment(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let step_port = context.listen("step", row, col - 1, '1');
    let mod_port = context.listen("mod", row, col + 1, 'z');

    let (step, _) = char_to_base_36(step_port.value);
    let (increment_mod, mod_upper) = char_to_base_36(mod_port.value);
    let increment_mod = increment_mod.max(1);
    let mut out_port = context.listen("out", row + 1, col, '0');
    let (out, _) = char_to_base_36(out_port.value);
    let out = (out + step) % increment_mod;
    out_port.value = base_36_to_char(out, mod_upper);

    vec![
        Update::Inputs(vec![step_port, mod_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn jump(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let input_port = context.listen("input", row - 1, col, '\0');
    let output_port = Port::new("output", row + 1, col, input_port.value);

    vec![
        Update::Inputs(vec![input_port]),
        Update::Outputs(vec![output_port]),
    ]
}

fn jymp(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let input_port = context.listen("input", row, col - 1, '\0');
    let output_port = Port::new("output", row, col + 1, input_port.value);

    vec![
        Update::Inputs(vec![input_port]),
        Update::Outputs(vec![output_port]),
    ]
}

fn lesser(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let a_port = context.listen("a", row, col - 1, '\0');
    let b_port = context.listen("b", row, col + 1, '\0');

    let out = if a_port.value != '\0' && b_port.value != '\0' {
        let (a, a_upper) = char_to_base_36(a_port.value);
        let (b, b_upper) = char_to_base_36(b_port.value);
        let less = if a < b { a } else { b };
        base_36_to_char(less, a_upper || b_upper)
    } else {
        '\0'
    };

    let out_port = Port::new("out", row + 1, col, out);

    vec![
        Update::Inputs(vec![a_port, b_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn multiply(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let a_port = context.listen("a", row, col - 1, '0');
    let b_port = context.listen("b", row, col + 1, '0');

    let (a, a_upper) = char_to_base_36(a_port.value);
    let (b, b_upper) = char_to_base_36(b_port.value);
    let out = base_36_to_char(a.saturating_mul(b), a_upper || b_upper);

    let out_port = Port::new("out", row + 1, col, out);

    vec![
        Update::Inputs(vec![a_port, b_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn read(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let x_port = context.listen("x", row, col - 2, '0');
    let y_port = context.listen("y", row, col - 1, '0');

    let (x, _) = char_to_base_36(x_port.value);
    let (y, _) = char_to_base_36(y_port.value);
    let val_port = context.listen("val", row + y as i32, col + 1 + x as i32, '\0');
    let out = val_port.value;

    let out_port = Port::new("out", row + 1, col, out);

    vec![
        Update::Inputs(vec![x_port, y_port, val_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn push(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let key_port = context.listen("key", row, col - 2, '0');
    let len_port = context.listen("len", row, col - 1, '1');

    let (key, _) = char_to_base_36(key_port.value);
    let (len, _) = char_to_base_36(len_port.value);
    let len = len.max(1);
    let val_port = context.listen("val", row, col + 1, '\0');
    let out = val_port.value;

    let out_port = Port::new("out", row + 1, col + (key % len) as i32, out);
    let locks = (0..(len as i32))
        .map(|i| Port::new("locked", row + 1, col + i, '\0'))
        .collect();

    vec![
        Update::Inputs(vec![key_port, len_port, val_port]),
        Update::Outputs(vec![out_port]),
        Update::Locks(locks),
    ]
}

fn query(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let x_port = context.listen("x", row, col - 3, '0');
    let y_port = context.listen("y", row, col - 2, '0');
    let len_port = context.listen("len", row, col - 1, '1');

    let (x, _) = char_to_base_36(x_port.value);
    let (y, _) = char_to_base_36(y_port.value);
    let (len, _) = char_to_base_36(len_port.value);
    let len = len.max(1);
    let mut input_ports: Vec<Port> = (0..len)
        .map(|i| {
            context.listen(
                &format!("in-{}", i),
                row + y as i32,
                col + 1 + x as i32 + i as i32,
                '\0',
            )
        })
        .collect();
    let output_ports = input_ports
        .iter()
        .enumerate()
        .map(|(i, port)| {
            Port::new(
                &format!("out-{}", i),
                row + 1,
                col + 1 + i as i32 - len as i32,
                port.value,
            )
        })
        .collect();

    input_ports.extend(vec![x_port, y_port]);
    vec![Update::Inputs(input_ports), Update::Outputs(output_ports)]
}

fn generate(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let len_port = context.listen("len", row, col - 1, '1');
    let y_port = context.listen("y", row, col - 2, '0');
    let x_port = context.listen("x", row, col - 3, '0');

    let (x, _) = char_to_base_36(x_port.value);
    let (y, _) = char_to_base_36(y_port.value);
    let (len, _) = char_to_base_36(len_port.value);
    let len = len.max(1);
    let mut input_ports: Vec<Port> = (0..len)
        .map(|i| context.listen(&format!("in-{}", i), row, col + 1 + i as i32, '\0'))
        .collect();
    let output_ports = input_ports
        .iter()
        .enumerate()
        .map(|(i, port)| {
            Port::new(
                &format!("out-{}", i),
                row + 1 + y as i32,
                col + i as i32 + x as i32,
                port.value,
            )
        })
        .collect();

    input_ports.extend(vec![x_port, y_port]);
    vec![Update::Inputs(input_ports), Update::Outputs(output_ports)]
}

fn write(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let x_port = context.listen("x", row, col - 2, '0');
    let y_port = context.listen("y", row, col - 1, '0');

    let (x, _) = char_to_base_36(x_port.value);
    let (y, _) = char_to_base_36(y_port.value);
    let val_port = context.listen("val", row, col + 1, '\0');
    let out = val_port.value;

    let out_port = Port::new("out", row + 1 + y as i32, col + x as i32, out);

    vec![
        Update::Inputs(vec![x_port, y_port, val_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn interpolate(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let rate_port = context.listen("rate", row, col - 1, '1');
    let target_port = context.listen("target", row, col + 1, 'z');

    let (rate, _) = char_to_base_36(rate_port.value);
    let (target, target_upper) = char_to_base_36(target_port.value);
    let mut out_port = context.listen("out", row + 1, col, '0');
    let (out, _) = char_to_base_36(out_port.value);
    let out = (out + rate).min(target);
    out_port.value = base_36_to_char(out, target_upper);

    vec![
        Update::Inputs(vec![rate_port, target_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn euclid(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let step_port = context.listen("density", row, col - 1, '1');
    let max_port = context.listen("length", row, col + 1, '8');
    let offset_port = context.listen("rotation", row, col + 2, '0');

    let (step, _) = char_to_base_36(step_port.value);
    let (max, _) = char_to_base_36(max_port.value);
    let (offset, _) = char_to_base_36(offset_port.value);
    let max = max.max(1);

    let mut out_port = context.listen("out", row + 1, col, '\0');
    if ((step as usize * (context.ticks + offset as usize)) % max as usize) < step as usize {
        out_port.value = '*';
    }

    vec![
        Update::Inputs(vec![step_port, max_port, offset_port]),
        Update::Outputs(vec![out_port]),
    ]
}

fn comment(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let width = context.cols as i32;
    let mut c = col + 1;
    for i in c..width {
        c = i;
        if context.read(row, c) == '#' {
            break;
        }
    }
    let locks = (col..(c + 1))
        .map(|l| Port::new("locked", row, l, '\0'))
        .collect();
    vec![Update::Locks(locks)]
}

fn variable(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let write_port = context.listen("write", row, col - 1, '.');
    let read_port = context.listen("read", row, col + 1, '.');

    if write_port.value == '.' {
        let out_port = Port::new("out", row + 1, col, context.read_variable(read_port.value));
        vec![
            Update::Inputs(vec![write_port, read_port]),
            Update::Outputs(vec![out_port]),
        ]
    } else {
        let value = read_port.value;
        vec![
            Update::Inputs(vec![read_port]),
            Update::Variables(vec![(write_port.value, value)]),
        ]
    }
}

fn bernoulli(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let propability_port = context.listen("num", row, col + 1, '2');

    let (probability, _) = char_to_base_36(propability_port.value);
    let mut out_port_zero = context.listen("out", row + 1, col, '\0');
    let mut out_port_one = context.listen("out2", row + 2, col, '\0');

    let d = Bernoulli::new(probability as f64 / 10.0).expect("invalid probability");
    let c = d.sample(&mut thread_rng());

    if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        if c && out_port_zero.value == '\0' {
            out_port_one.value = '*';
        }

        if out_port_one.value == '\0' {
            out_port_zero.value = '*'
        }
    }
    vec![
        Update::Inputs(vec![propability_port]),
        Update::Outputs(vec![out_port_zero, out_port_one]),
    ]
}

fn concat(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let len_port = context.listen("len", row, col - 1, '1');

    let (len, _) = char_to_base_36(len_port.value);
    let output_ports = (0..(len as i32))
        .map(|i| {
            Port::new(
                &format!("out-{}", i),
                row + 1,
                col + i + 1,
                context.read_variable(context.read(row, col + i + 1)),
            )
        })
        .collect();
    let locks = (0..(len as i32))
        .map(|i| Port::new("locked", row, col + 1 + i, '\0'))
        .collect();
    vec![
        Update::Inputs(vec![len_port]),
        Update::Outputs(output_ports),
        Update::Locks(locks),
    ]
}

pub fn saver(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let key_port_one = context.listen("ch1", row, col + 1, '.');
    let key_port_two = context.listen("ch2", row, col + 2, '.');
    let key_port_three = context.listen("ch3", row, col + 3, '.');
    let key_port_four = context.listen("ch4", row, col + 4, '.');
    let key_port_five = context.listen("ch5", row, col + 5, '.');
    let key_port_six = context.listen("ch6", row, col + 6, '.');
    let key_port_seven = context.listen("ch7", row, col + 7, '.');
    let key_port_eight = context.listen("ch8", row, col + 8, '.');

    let mut name = String::new();
    name.push(key_port_one.value);
    name.push(key_port_two.value);
    name.push(key_port_three.value);
    name.push(key_port_four.value);
    name.push(key_port_five.value);
    name.push(key_port_six.value);
    name.push(key_port_seven.value);
    name.push(key_port_eight.value);
    let name: String = name.chars().filter(|a| a.is_alphanumeric()).collect();
    let locks = (0..8)
        .map(|i| Port::new("locked", row, col + 1 + i, '\0'))
        .collect();
    let output = if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        name.clone()
    } else {
        "buffer".to_string()
    };

    vec![
        Update::Inputs(vec![
            key_port_one,
            key_port_two,
            key_port_three,
            key_port_four,
            key_port_five,
            key_port_six,
            key_port_seven,
            key_port_eight,
        ]),
        Update::Locks(locks),
        Update::Save(output.trim_matches('.').to_string()),
    ]
}

pub fn snippet_saver(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let key_port_one = context.listen("ch1", row, col + 1, '.');
    let key_port_two = context.listen("ch2", row, col + 2, '.');
    let key_port_three = context.listen("ch3", row, col + 3, '.');
    let key_port_four = context.listen("ch4", row, col + 4, '.');
    let key_port_five = context.listen("ch5", row, col + 5, '.');
    let key_port_six = context.listen("ch6", row, col + 6, '.');
    let key_port_seven = context.listen("ch7", row, col + 7, '.');
    let key_port_eight = context.listen("ch8", row, col + 8, '.');

    let mut name = String::new();
    name.push(key_port_one.value);
    name.push(key_port_two.value);
    name.push(key_port_three.value);
    name.push(key_port_four.value);
    name.push(key_port_five.value);
    name.push(key_port_six.value);
    name.push(key_port_seven.value);
    name.push(key_port_eight.value);
    let locks = (0..8)
        .map(|i| Port::new("locked", row, col + 1 + i, '\0'))
        .collect();
    if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        let name = name.clone();
        let dir_path = Path::new("orca/snippets");

        // Check if directory exists, if not create it
        if !dir_path.exists() {
            fs::create_dir_all(dir_path).expect("Failed to create directory");
        }

        let file_path = dir_path.join(name.trim_matches('.'));

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_path)
            .expect("Failed to open file");

        let mut clipboard = ClipboardContext::new().expect("Failed to get clipboard context");

        let cells_to_paste: Vec<Vec<char>> = clipboard
            .get_contents()
            .expect("Failed to get clipboard contents")
            .split('\n')
            .map(|row| row.chars().collect())
            .collect();

        for row in cells_to_paste {
            let row_string: String = row.into_iter().collect();
            file.write_all(row_string.as_bytes()).expect("Failed to write to file");
            file.write_all(b"\n").expect("Failed to write to file");
        }
    } else {
        "snippet".to_string();
    };

    vec![
        Update::Inputs(vec![
            key_port_one,
            key_port_two,
            key_port_three,
            key_port_four,
            key_port_five,
            key_port_six,
            key_port_seven,
            key_port_eight,
        ]),
        Update::Locks(locks),
    ]
}

pub fn loader(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let key_port_one = context.listen("ch1", row, col + 1, '.');
    let key_port_two = context.listen("ch2", row, col + 2, '.');
    let key_port_three = context.listen("ch3", row, col + 3, '.');
    let key_port_four = context.listen("ch4", row, col + 4, '.');
    let key_port_five = context.listen("ch5", row, col + 5, '.');
    let key_port_six = context.listen("ch6", row, col + 6, '.');
    let key_port_seven = context.listen("ch7", row, col + 7, '.');
    let key_port_eight = context.listen("ch8", row, col + 8, '.');

    let mut name = String::new();
    name.push(key_port_one.value);
    name.push(key_port_two.value);
    name.push(key_port_three.value);
    name.push(key_port_four.value);
    name.push(key_port_five.value);
    name.push(key_port_six.value);
    name.push(key_port_seven.value);
    name.push(key_port_eight.value);
    let locks = (0..8)
        .map(|i| Port::new("locked", row, col + 1 + i, '\0'))
        .collect();

    let output = if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        name.trim_matches('.').to_string().clone()
    } else {
        "buffer".to_string()
    };

    vec![
        Update::Inputs(vec![
            key_port_one,
            key_port_two,
            key_port_three,
            key_port_four,
            key_port_five,
            key_port_six,
            key_port_seven,
            key_port_eight,
        ]),
        Update::Locks(locks),
        Update::Load(output),
    ]
}

pub fn snippet_loader(context: &Context, row: i32, col: i32) -> Vec<Update> {
    let key_port_one = context.listen("ch1", row, col + 1, '.');
    let key_port_two = context.listen("ch2", row, col + 2, '.');
    let key_port_three = context.listen("ch3", row, col + 3, '.');
    let key_port_four = context.listen("ch4", row, col + 4, '.');
    let key_port_five = context.listen("ch5", row, col + 5, '.');
    let key_port_six = context.listen("ch6", row, col + 6, '.');
    let key_port_seven = context.listen("ch7", row, col + 7, '.');
    let key_port_eight = context.listen("ch8", row, col + 8, '.');

    let mut name = String::new();
    name.push(key_port_one.value);
    name.push(key_port_two.value);
    name.push(key_port_three.value);
    name.push(key_port_four.value);
    name.push(key_port_five.value);
    name.push(key_port_six.value);
    name.push(key_port_seven.value);
    name.push(key_port_eight.value);
    let locks = (0..8)
        .map(|i| Port::new("locked", row, col + 1 + i, '\0'))
        .collect();
    if context.read(row - 1, col) == '*'
        || context.read(row, col - 1) == '*'
        || context.read(row + 1, col) == '*'
    {
        let name = name.clone();
        let dir_path = Path::new("orca/snippets");

        if !dir_path.exists() {
            fs::create_dir_all(dir_path).expect("Failed to create directory");
        }

        let file_path = dir_path.join(name.trim_matches('.'));

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_path)
            .expect("Failed to open file");

        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read file");

        let mut clipboard = ClipboardContext::new().expect("Failed to get clipboard context");
        clipboard.set_contents(contents.to_owned()).expect("Failed to set clipboard contents");
    }

    vec![
        Update::Inputs(vec![
            key_port_one,
            key_port_two,
            key_port_three,
            key_port_four,
            key_port_five,
            key_port_six,
            key_port_seven,
            key_port_eight,
        ]),
        Update::Locks(locks),
    ]
}

pub fn get_bang_operators(operator_map: &HashMap<String, char>) -> HashMap<char, Operator> {
    let mut operators: HashMap<char, Operator> = HashMap::new();
    for (c, operator) in get_tick_operators(operator_map) {
        operators.insert(c.to_ascii_lowercase(), operator);
    }
    operators
}

pub fn grid_tick(
    context: &mut Context,
    tick_operators: &HashMap<char, Operator>,
    bang_operators: &HashMap<char, Operator>,
    should_redraw_midi: Arc<AtomicBool>,
) {
    let rows = context.rows as i32;
    let cols = context.cols as i32;
    context.unlock_all();
    context.clear_all_variables();

    // clear previous bangs
    for row in 0..rows {
        for col in 0..cols {
            if context.read(row, col) == '*' {
                context.write(row, col, '.');
            }
        }
    }

    // apply grid operators (which may produce new bangs)
    for row in 0..rows {
        for col in 0..cols {
            if let Some(operator) = tick_operators.get(&context.read(row, col)) {
                operator.apply(context, row, col);
                should_redraw_midi.store(true, Ordering::Relaxed);
            }
        }
    }

    // apply bang operators on current bangs
    for row in 0..rows {
        for col in 0..cols {
            if let Some(operator) = bang_operators.get(&context.read(row, col)) {
                if context.read(row - 1, col) == '*'
                    || context.read(row, col - 1) == '*'
                    || context.read(row + 1, col) == '*'
                {
                    operator.apply(context, row, col);
                }
            }
        }
    }

    context.ticks += 1;
}
