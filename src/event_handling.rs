use std::{fs, fs::OpenOptions, io::Write, sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
}};
use std::path::Path;

use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::disable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::context::{AppState, Context, Mode};
use crate::{Cursor, RowsCols};

pub fn handle_events(
    should_redraw: &Arc<AtomicBool>,
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mode: &mut Mode,
    selected_cells: &mut Option<Vec<Vec<char>>>,
    cursor: &mut Cursor,
    show_popup: &mut bool,
    rows_cols: &RowsCols,
) {
    match crossterm::event::read().expect("Failed to read event") {
        Event::Key(KeyEvent {
                       code, modifiers, ..
                   }) => {
            should_redraw.store(true, Ordering::Relaxed);
            match code {
                KeyCode::Char('=') => {
                    tempo_up(context_arc);
                }

                KeyCode::Char('-') => {
                    tempo_down(context_arc);
                }

                KeyCode::Char('q') if modifiers == KeyModifiers::CONTROL => {
                    quit(context_arc, terminal);
                }

                KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => {
                    copy(mode, context_arc, selected_cells);
                }

                KeyCode::Char('v') if modifiers == KeyModifiers::CONTROL => {
                    paste(context_arc, *cursor.cursor_row, *cursor.cursor_col, mode);
                }

                KeyCode::Char('h') if modifiers == KeyModifiers::CONTROL => {
                    *show_popup = !*show_popup;
                }

                KeyCode::Char('d') if modifiers == KeyModifiers::CONTROL => {
                    clear_grid(context_arc, rows_cols.rows, rows_cols.cols);
                }

                KeyCode::Char(' ') => {
                    pause(context_arc);
                }

                KeyCode::Char('p') if modifiers == KeyModifiers::CONTROL => {
                    change_midi_port(context_arc);
                }

                KeyCode::Up => {
                    *show_popup = false;
                    cursor_up(
                        cursor.cursor_row,
                        mode,
                        &*selected_cells,
                        context_arc,
                        *cursor.cursor_col
                    );
                }

                KeyCode::Down => {
                    *show_popup = false;
                    cursor_down(
                        cursor.cursor_row,
                        mode,
                        rows_cols.rows,
                        &*selected_cells,
                        context_arc,
                        *cursor.cursor_col,
                    );
                }

                KeyCode::Left => {
                    *show_popup = false;
                    cursor_left(
                        cursor.cursor_col,
                        mode,
                        &*selected_cells,
                        context_arc,
                        *cursor.cursor_row
                    );
                }

                KeyCode::Right => {
                    *show_popup = false;
                    cursor_right(
                        cursor.cursor_col,
                        mode,
                        rows_cols.cols,
                        &*selected_cells,
                        context_arc,
                        *cursor.cursor_row,
                    );
                }

                KeyCode::Char(c) => {
                    input_char(
                        c,
                        mode,
                        cursor.cursor_row,
                        cursor.cursor_col,
                        context_arc,
                        selected_cells
                    );
                }

                KeyCode::Esc => {
                    *show_popup = false;
                    escape(mode);
                }

                KeyCode::Backspace => {
                    backspace(mode, context_arc, *cursor.cursor_row, *cursor.cursor_col);
                }
                _ => {}
            }
        }

        _ => {}
    }
}

pub fn cursor_up(
    cursor_row: &mut usize,
    mode: &mut Mode,
    selected_cells: &Option<Vec<Vec<char>>>,
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    cursor_col: usize,
) {
    if let Mode::Move = mode {
        if *cursor_row > 0 {
            *cursor_row -= 1;
            if let Some(ref cells) = *selected_cells {
                let mut context = context_arc.lock();
                let max_row_index = context.grid.len() - 1;
                let max_col_index = context.grid[0].len() - 1;
                for (r, row) in cells.iter().enumerate() {
                    for (c, &value) in row.iter().enumerate() {
                        let target_row = *cursor_row + r;
                        let target_col = cursor_col + c;
                        if target_row <= max_row_index && target_col <= max_col_index {
                            context.grid[target_row][target_col] = value;
                        }
                    }
                }
            }
        }
    } else {
        *cursor_row = cursor_row.saturating_sub(1);
        if let Mode::Select { start: _, end } = mode {
            *end = (*cursor_row, cursor_col);
        }
    }
}

pub fn cursor_down(
    cursor_row: &mut usize,
    mode: &mut Mode,
    rows: usize,
    selected_cells: &Option<Vec<Vec<char>>>,
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    cursor_col: usize,
) {
    if let Mode::Move = mode {
        if *cursor_row < rows - 1 {
            *cursor_row += 1;
            if let Some(ref cells) = *selected_cells {
                let mut context = context_arc.lock();
                let max_row_index = context.grid.len() - 1;
                let max_col_index = context.grid[0].len() - 1;
                for (r, row) in cells.iter().enumerate() {
                    for (c, &value) in row.iter().enumerate() {
                        let target_row = *cursor_row + r;
                        let target_col = cursor_col + c;
                        if target_row <= max_row_index && target_col <= max_col_index {
                            context.grid[target_row][target_col] = value;
                        }
                    }
                }
            }
        }
    } else {
        if *cursor_row < rows - 1 {
            *cursor_row += 1;
        }
        if let Mode::Select { start: _, end } = mode {
            *end = (*cursor_row, cursor_col);
        }
    }
}

pub fn cursor_left(
    cursor_col: &mut usize,
    mode: &mut Mode,
    selected_cells: &Option<Vec<Vec<char>>>,
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    cursor_row: usize,
) {
    if let Mode::Move = mode {
        if *cursor_col > 0 {
            *cursor_col -= 1;
            if let Some(ref cells) = *selected_cells {
                let mut context = context_arc.lock();
                let max_row_index = context.grid.len() - 1;
                let max_col_index = context.grid[0].len() - 1;
                for (r, row) in cells.iter().enumerate() {
                    for (c, &value) in row.iter().enumerate() {
                        let target_row = cursor_row + r;
                        let target_col = *cursor_col + c;
                        if target_row <= max_row_index && target_col <= max_col_index {
                            context.grid[target_row][target_col] = value;
                        }
                    }
                }
            }
        }
    } else {
        *cursor_col = cursor_col.saturating_sub(1);
        if let Mode::Select { start: _, end } = mode {
            *end = (cursor_row, *cursor_col);
        }
    }
}

pub fn cursor_right(
    cursor_col: &mut usize,
    mode: &mut Mode,
    cols: usize,
    selected_cells: &Option<Vec<Vec<char>>>,
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    cursor_row: usize,
) {
    if let Mode::Move = mode {
        if *cursor_col < cols - 1 {
            *cursor_col += 1;
            if let Some(ref cells) = *selected_cells {
                let mut context = context_arc.lock();
                let max_row_index = context.grid.len() - 1;
                let max_col_index = context.grid[0].len() - 1;
                for (r, row) in cells.iter().enumerate() {
                    for (c, &value) in row.iter().enumerate() {
                        let target_row = cursor_row + r;
                        let target_col = *cursor_col + c;
                        if target_row <= max_row_index && target_col <= max_col_index {
                            context.grid[target_row][target_col] = value;
                        }
                    }
                }
            }
        }
    } else {
        if *cursor_col < cols - 1 {
            *cursor_col += 1;
        }
        if let Mode::Select { start: _, end } = mode {
            *end = (cursor_row, *cursor_col);
        }
    }
}

pub fn input_char(
    c: char,
    mode: &mut Mode,
    cursor_row: &mut usize,
    cursor_col: &mut usize,
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    selected_cells: &mut Option<Vec<Vec<char>>>,
) {
    if c == '`' {
        match *mode {
            Mode::Normal => {
                *mode = Mode::Select {
                    start: (*cursor_row, *cursor_col),
                    end: (*cursor_row, *cursor_col),
                };
            }
            Mode::Select { .. } => {
                *mode = Mode::Normal;
            }
            _ => {}
        }
    } else if c == '/' {
        if let Mode::Select { start, end } = *mode {
            let context = context_arc.lock();
            let min_row = start.0.min(end.0);
            let max_row = start.0.max(end.0);
            let min_col = start.1.min(end.1);
            let max_col = start.1.max(end.1);

            let mut moved_cells = vec![];

            for row in min_row..=max_row {
                let mut moved_row = vec![];
                for col in min_col..=max_col {
                    moved_row.push(context.grid[row][col]);
                }
                moved_cells.push(moved_row);
            }

            *selected_cells = Some(moved_cells);
            *mode = Mode::Move;
            *cursor_row = min_row;
            *cursor_col = min_col;
        }
    } else {
        let mut _context = context_arc.lock();
        _context.grid[*cursor_row][*cursor_col] = c;
    }
}

pub fn backspace(
    mode: &mut Mode,
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    cursor_row: usize,
    cursor_col: usize,
) {
    if let Mode::Select { start, end } = *mode {
        let mut context = context_arc.lock();
        let min_row = start.0.min(end.0);
        let max_row = start.0.max(end.0);
        let min_col = start.1.min(end.1);
        let max_col = start.1.max(end.1);

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                context.grid[row][col] = '.';
            }
        }
        *mode = Mode::Normal;
    } else {
        let mut _context = context_arc.lock();
        _context.grid[cursor_row][cursor_col] = '.';
    }
}

pub fn clear_grid(
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    rows: usize,
    cols: usize,
) {
    let mut context = context_arc.lock();
    context.grid = (0..rows)
        .map(|_| (0..cols).map(|_| '.').collect())
        .collect();
}

pub fn copy(
    mode: &mut Mode,
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    selected_cells: &mut Option<Vec<Vec<char>>>,
) {
    if let Mode::Select { start, end } = *mode {
        let context = context_arc.lock();
        let min_row = start.0.min(end.0);
        let max_row = start.0.max(end.0);
        let min_col = start.1.min(end.1);
        let max_col = start.1.max(end.1);

        let mut copied_cells = vec![];

        for row in min_row..=max_row {
            let mut copied_row = vec![];
            for col in min_col..=max_col {
                copied_row.push(context.grid[row][col]);
            }
            copied_cells.push(copied_row);
        }
        let copy = copied_cells.clone();
        let clip: String = copy
            .into_iter()
            .map(|c_vec| c_vec.into_iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\r\n");

        let mut clipboard = ClipboardContext::new().expect("Failed to get clipboard");
        clipboard.set_contents(clip.to_owned()).expect("Failed to set clipboard");
        *selected_cells = Some(copied_cells);
        *mode = Mode::Copy;
    }
}

pub fn paste(
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    cursor_row: usize,
    cursor_col: usize,
    mode: &mut Mode,
) {
    let mut clipboard = ClipboardContext::new().expect("Failed to get clipboard");
    let cells_to_paste: Vec<Vec<char>> = clipboard
        .get_contents()
        .expect("Failed to get clipboard contents")
        .split('\n')
        .map(|row| row.chars().filter(|c| !c.is_whitespace()).collect())
        .collect();

    if let cells = cells_to_paste {
        let mut _context = context_arc.lock();
        let max_row_index = _context.grid.len() - 1;
        let max_col_index = _context.grid[0].len() - 1;

        for (r, row) in cells.iter().enumerate() {
            for (c, &value) in row.iter().enumerate() {
                let target_row = cursor_row + r;
                let target_col = cursor_col + c + 1;

                // Only paste cells within the grid boundaries
                if target_row <= max_row_index && target_col <= max_col_index {
                    _context.grid[target_row][target_col] = value;
                }
            }
        }
    }
    *mode = Mode::Normal;
}

pub fn pause(context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>) {
    let mut context = context_arc.lock();
    if context.app_state == AppState::Running {
        context.app_state = AppState::Paused;
    } else {
        context.app_state = AppState::Running;
    }
}

pub fn quit(
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) {
    let dir_path = Path::new("orca/sessions");
    if !dir_path.exists() {
        fs::create_dir_all(dir_path).expect("Unable to create directory");
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("orca/sessions/last_session")
        .expect("Unable to save file");

    let grid = { context_arc.lock().grid.clone() };

    for row in grid {
        let row_string: String = row.into_iter().collect();
        file.write_all(row_string.as_bytes()).expect("Unable to write file");
        file.write_all(b"\n").expect("Unable to write file");
    }
    disable_raw_mode().unwrap();
    terminal.show_cursor().unwrap();
    terminal.clear().unwrap();
    std::process::exit(0);
}

// controls

pub fn change_midi_port(
    context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>,
) {
    let mut context = context_arc.lock();
    context.midi_port += 1;
}

pub fn escape(mode: &mut Mode) {
    match *mode {
        Mode::Select { .. } | Mode::Copy | Mode::Move => {
            *mode = Mode::Normal;
        }
        _ => {}
    }
}

pub fn tempo_up(context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>) {
    let mut context = context_arc.lock();
    context.tempo += 1;
}

pub fn tempo_down(context_arc: &Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, Context>>) {
    let mut context = context_arc.lock();
    if context.tempo > 1 {
        context.tempo -= 1;
    }
}
