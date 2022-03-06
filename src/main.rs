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
use crate::operators::grid_tick;

fn main() {
    let rows = 30;
    let cols = 100;
    let grid: Vec<Vec<char>> = (0..rows).map(|_| (0..cols).map(|_| '.').collect()).collect();
    let mut context = Context::new(grid, 120, 4);

    let context_arc = Arc::new(Mutex::new(context));
    let midi_context_arc = Arc::clone(&context_arc);

    thread::spawn(move || {
        let tick_operators = operators::get_tick_operators();
        let bang_operators = operators::get_bang_operators();
        let midi_out = MidiOutput::new("rust-orca").unwrap();
        let out_ports = midi_out.ports();
        let out_port = out_ports.get(2).unwrap();
        let mut conn = &mut midi_out.connect(out_port, "rust-orca-conn").unwrap();

        // clear all existing midi notes
        // TODO clear existing midi notes when program is closed as well
        for channel in 0..16 {
            for note in 0..128 {
                conn.send(&[0x80 + channel, note, 0]);
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


    let (mut row, mut col): (usize, usize) = (0, 0);

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
        for row in grid.iter() {
            for chr in row.iter() {
                window.addch(*chr);
            }
        }
        window.mv(row as i32, col as i32);

        match window.getch() {
            Some(input) => {
                match input {
                    Input::KeyUp => { row -= 1; }
                    Input::KeyDown => { row += 1; }
                    Input::KeyLeft => { col -= 1; }
                    Input::KeyRight => { col += 1; }
                    Input::KeyBackspace => {
                        let mut _context = context_arc.lock().unwrap();
                        _context.grid[row][col] = '.';
                    }
                    Input::KeyDC => {
                        let mut _context = context_arc.lock().unwrap();
                        _context.grid[row][col] = '.';
                    }
                    Input::KeyMouse => {
                        if let Ok(mouse_event) = getmouse() {
                            row = mouse_event.y as usize;
                            col = mouse_event.x as usize;
                        }
                    }
                    Input::Character(mut c) => {
                        if c == '\x08' {
                            c = '.';
                        }
                        window.addch(c);
                        let mut _context = context_arc.lock().unwrap();
                        _context.grid[row][col] = c;
                    }
                    input => { println!("unexpected input: {:?}", input); }
                }
            }
            None => ()
        }

        sleep(Duration::from_millis(10));
    }
}