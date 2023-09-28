use std::{
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    time::Duration,
};
use crate::{
    context::{Context, Mode},
    midi::{run_midi, run_midi_cc},
    note_events::{run_notes, Note},
    sampler::sampler_out,
    synth::synth_out,
};
use crossbeam::channel::{unbounded, Sender};
use crossterm::{event::poll, terminal::enable_raw_mode};
use parking_lot::Mutex;
use ratatui::{backend::CrosstermBackend, Terminal};

mod context;
mod event_handling;
mod midi;
mod note_events;
mod operators;
mod sampler;
mod synth;
mod ui;
mod utils;

pub struct NoteSenders {
    midi_note_sender: Sender<Vec<Note>>,
    sampler_note_sender: Sender<Vec<Note>>,
    midi_cc_sender: Sender<Vec<Note>>,
    synth_note_sender: Sender<Vec<Note>>,
}

pub struct RowsCols {
    rows: usize,
    cols: usize,
}

pub struct Cursor<'a> {
    cursor_row: &'a mut usize,
    cursor_col: &'a mut usize,
}

fn main() {
    // get arguments
    let args: Vec<String> = std::env::args().skip(1).collect();
    // prepare terminal
    let stdout = std::io::stdout();
    enable_raw_mode().unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.clear().unwrap();

    // prepare context
    let mut cursor = Cursor {
        cursor_row: &mut 0,
        cursor_col: &mut 0,
    };
    let mut selected_cells: Option<Vec<Vec<char>>> = None;
    let mut mode = Mode::Normal;
    let rows_cols = RowsCols {
        rows: args.get(1).unwrap_or(&"50".to_string()).parse().unwrap(),
        cols: args.get(2).unwrap_or(&"150".to_string()).parse().unwrap(),
    };
    let new_or_last: String = args.get(0).unwrap_or(&"new".to_string()).parse().unwrap();

    let context = Context::new(110, 4, rows_cols.rows, rows_cols.cols, &new_or_last);
    let should_redraw = Arc::new(AtomicBool::new(true));
    let should_redraw_notes = Arc::clone(&should_redraw);
    let context_arc = Arc::new(Mutex::new(context));
    let notes_context_arc = Arc::clone(&context_arc);
    let midi_context_arc = Arc::clone(&context_arc);

    // prepare channels
    let (midi_note_sender, midi_note_receiver) = unbounded();
    let (midi_cc_sender, midi_cc_receiver) = unbounded();
    let (midi_port_sender, midi_port_receiver) = unbounded();
    let (sampler_note_sender, sampler_note_receiver) = unbounded();
    let (synth_note_sender, synth_note_receiver) = unbounded();
    let mut show_popup = true;

    let note_senders = NoteSenders {
        midi_note_sender,
        sampler_note_sender,
        midi_cc_sender,
        synth_note_sender,
    };

    // run note events
    run_notes(
        notes_context_arc,
        should_redraw_notes,
        note_senders,
        midi_port_sender,
    );

    // run synth thread
    synth_out(synth_note_receiver);

    // run sampler thread
    sampler_out(sampler_note_receiver);

    // run MIDI thread
    run_midi(
        midi_note_receiver,
        midi_port_receiver,
        midi_context_arc,
    );

    run_midi_cc(midi_cc_receiver);

    // run TUI
    loop {
        if should_redraw.load(Ordering::Relaxed) {
            ui::draw(
                &mut terminal,
                &cursor,
                &mut mode,
                &should_redraw,
                &context_arc,
                show_popup,
            );
        }

        if poll(Duration::from_millis(10)).unwrap() {
            event_handling::handle_events(
                &should_redraw,
                &context_arc,
                &mut terminal,
                &mut mode,
                &mut selected_cells,
                &mut cursor,
                &mut show_popup,
                &rows_cols,
            );
        }
    }
}
