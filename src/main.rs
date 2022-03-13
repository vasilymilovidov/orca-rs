mod context;
mod midi;
mod operators;

use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};
use midir::MidiOutput;
use pancurses::{ALL_MOUSE_EVENTS, cbreak, curs_set, getmouse, initscr, Input, mousemask, noecho, resize_term};
use crate::context::Context;
use crate::midi::notes_tick;
use crate::operators::{get_bang_operators, get_tick_operators, grid_tick, read_operator_config};

fn main() {
    let rows = 30;
    let cols = 100;
    let grid_row_spacing = 9;
    let grid_col_spacing = 9;
    let grid: Vec<Vec<char>> = (0..rows).map(|_| (0..cols).map(|_| '\0').collect()).collect();
    let context = Context::new(grid, 120, 4);

    let context_arc = Arc::new(Mutex::new(context));
    let midi_context_arc = Arc::clone(&context_arc);

    thread::spawn(move || {
        let operator_map = read_operator_config("operator_config.txt");
        let tick_operators = operators::get_tick_operators(&operator_map);
        let bang_operators = operators::get_bang_operators(&operator_map);
        let midi_out = MidiOutput::new("rust-orca").unwrap();
        let out_ports = midi_out.ports();
        let out_port = out_ports.get(2).unwrap();
        let conn = &mut midi_out.connect(out_port, "rust-orca-conn").unwrap();

        // clear all existing midi notes
        // TODO clear existing midi notes when program is closed as well
        for channel in 0..16 {
            for note in 0..128 {
                let note_off_message = 0x80 + channel;
                conn.send(&[note_off_message, note, 0]).unwrap();
            }
        }

        loop {
            let sleep_time = {
                let timer = Instant::now();

                let mut _context = midi_context_arc.lock().unwrap();
                grid_tick(&mut _context, &tick_operators, &bang_operators);

                let mut notes = notes_tick(&_context.notes, _context.tick_time);
                for note in notes.iter_mut() {
                    if note.started && note.duration == 0 {
                        note.stop(conn);
                    } else if !note.started {
                        note.stop(conn);
                        note.start(conn);
                    }
                }
                _context.notes = notes.iter().filter(|note| note.duration > 0).cloned().collect();

                let elapsed = timer.elapsed().as_secs_f64();
                60.0 / (_context.divisions * _context.tempo) as f64 - elapsed
            };

            if sleep_time > 0.0 {
                sleep(Duration::from_secs_f64(sleep_time));
            }
        }
    });


    let (mut cursor_row, mut cursor_col): (usize, usize) = (0, 0);

    let mut window = initscr();
    resize_term(rows, cols);
    cbreak();
    noecho();
    curs_set(2);
    mousemask(ALL_MOUSE_EVENTS, None);
    window.resize(rows, cols);
    window.keypad(true);
    window.nodelay(true);
    window.refresh();

    loop {
        // TODO use swap buffer with diffs to reduce latency
        let grid = {
            let _context = context_arc.lock().unwrap();
            _context.grid.clone()
        };
        window.mv(0, 0);
        for (r, row) in grid.iter().enumerate() {
            for (c, &value) in row.iter().enumerate() {
                let display_value = if value != '\0' {
                    value
                } else if r % grid_row_spacing == 0 && c % grid_col_spacing == 0 {
                    '+'
                } else {
                    ' '
                };
                window.addch(display_value);
            }
        }
        window.mv(cursor_row as i32, cursor_col as i32);

        match window.getch() {
            Some(input) => {
                match input {
                    Input::KeyUp => { cursor_row -= 1; }
                    Input::KeyDown => { cursor_row += 1; }
                    Input::KeyLeft => { cursor_col -= 1; }
                    Input::KeyRight => { cursor_col += 1; }
                    Input::KeyBackspace => {
                        let mut _context = context_arc.lock().unwrap();
                        _context.grid[cursor_row][cursor_col] = '\0';
                    }
                    Input::KeyDC => {
                        let mut _context = context_arc.lock().unwrap();
                        _context.grid[cursor_row][cursor_col] = '\0';
                    }
                    Input::KeyMouse => {
                        if let Ok(mouse_event) = getmouse() {
                            cursor_row = mouse_event.y as usize;
                            cursor_col = mouse_event.x as usize;
                        }
                    }
                    Input::Character(mut c) => {
                        if c == '\x08' {
                            c = '\0';
                        }
                        window.addch(c);
                        let mut _context = context_arc.lock().unwrap();
                        _context.grid[cursor_row][cursor_col] = c;
                    }
                    input => { println!("unexpected input: {:?}", input); }
                }
            }
            None => ()
        }

        sleep(Duration::from_millis(10));
    }
}