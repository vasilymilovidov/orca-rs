use crate::note_events::{MidiCC, Note};
use std::{collections::{HashMap, HashSet}, fs::{File, OpenOptions}, fs, io::{Read, Write}};
use std::path::Path;

#[derive(Copy, Clone)]
pub enum Mode {
    Normal,
    Select {
        start: (usize, usize),
        end: (usize, usize),
    },
    Copy,
    Move,
}

#[derive(PartialEq, Copy, Clone)]
pub enum AppState {
    Shutdown,
    Paused,
    Running,
}

#[derive(Clone, Debug)]
pub struct Port {
    pub name: String,
    pub row: i32,
    pub col: i32,
    pub value: char,
}

impl Port {
    pub fn new(name: &str, row: i32, col: i32, value: char) -> Port {
        Port {
            name: String::from(name),
            row,
            col,
            value,
        }
    }
}

pub struct Globals {
    pub global_key: char,
    pub global_scale: char,
}

pub struct Context {
    pub grid: Vec<Vec<char>>,
    pub notes: Vec<Note>,
    pub cc: Vec<MidiCC>,
    pub locks: HashSet<(i32, i32)>,
    pub variables: HashMap<char, char>,
    pub ticks: usize,
    pub tempo: u64,
    pub divisions: u64,
    pub tick_time: u64,
    pub app_state: AppState,
    pub ports: HashMap<(i32, i32), String>,
    pub rows: usize,
    pub cols: usize,
    pub global_scale: char,
    pub global_key: char,
    pub midi_port: u8,
    pub midi_port_name: String,
}

impl Context {
    pub fn new(tempo: u64, divisions: u64, rows: usize, cols: usize, new_or_last: &str) -> Context {
        // open last session or create a new empty grid
        let grid: Vec<Vec<char>>;

        if new_or_last == "last" {
            match File::open("last_session") {
                Ok(mut session) => {
                    let mut contents = String::new();
                    session.read_to_string(&mut contents).expect("Unable to read file");

                    grid = contents
                        .lines()
                        .map(|line| line.chars().collect())
                        .collect();
                }
                _ => {
                    grid = (0..rows)
                        .map(|_| (0..cols).map(|_| '.').collect())
                        .collect();
                }
            }
        } else {
            match File::open(new_or_last) {
                Ok(mut session) => {
                    let mut contents = String::new();
                    session.read_to_string(&mut contents).expect("Unable to read file");

                    grid = contents
                        .lines()
                        .map(|line| line.chars().collect())
                        .collect();
                }
                _ => {
                    grid = (0..rows)
                        .map(|_| (0..cols).map(|_| '.').collect())
                        .collect();
                }
            }
        };


        Context {
            grid,
            notes: Vec::new(),
            cc: Vec::new(),
            locks: HashSet::new(),
            variables: HashMap::new(),
            ticks: 0,
            tempo,
            divisions,
            tick_time: 60000 / (tempo * divisions),
            app_state: AppState::Running,
            ports: HashMap::new(),
            rows,
            cols,
            global_scale: '0',
            global_key: 'C',
            midi_port: 0,
            midi_port_name: String::new(),
        }
    }

    pub fn is_port(&self, row: usize, col: usize) -> bool {
        self.locks.contains(&(row as i32, col as i32))
    }

    pub fn read(&self, row: i32, col: i32) -> char {
        if row < 0 || col < 0 {
            return '\0';
        }

        let row = row as usize;
        let col = col as usize;

        self.grid
            .get(row)
            .and_then(|row| row.get(col).cloned())
            .unwrap_or('\0')
    }

    pub fn get_port_name(&self, row: usize, col: usize) -> Option<&String> {
        self.ports.get(&(row as i32, col as i32))
    }

    pub fn listen(&self, name: &str, row: i32, col: i32, default: char) -> Port {
        let value = self.read(row, col);
        let value = if value == '.' { default } else { value };
        Port::new(name, row, col, value)
    }

    pub fn write(&mut self, row: i32, col: i32, value: char) {
        if row < 0 || col < 0 {
            return;
        }

        let row = row as usize;
        let col = col as usize;

        if let Some(row) = self.grid.get_mut(row) {
            if let Some(cell) = row.get_mut(col) {
                *cell = value;
            }
        }
    }

    pub fn save(&mut self, name: String) {
        let dir_path = Path::new("orca/sessions");
        if !dir_path.exists() {
            fs::create_dir_all(dir_path).expect("Unable to create directory");
        }
        let file_name = format!("orca/sessions/{}", name.trim_matches('.'));
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_name)
            .expect("Unable to open file");

        let grid = self.grid.clone();

        for row in grid {
            let row_string: String = row.into_iter().collect();
            file.write_all(row_string.as_bytes()).expect("Unable to write file");
            file.write_all(b"\n").expect("Unable to write file");
        }
    }

    pub fn load(&mut self, name: String) {
        if name != "buffer" {
            let file_name = format!("orca/sessions/{}", name.trim_matches('.'));
            let mut file = File::open(file_name).unwrap_or(File::open("orca/sessions/buffer").expect("Unable to open file"));
            let mut contents = String::new();
            file.read_to_string(&mut contents).expect("Unable to read file");

            let grid: Vec<Vec<char>> = contents
                .lines()
                .map(|line| line.chars().collect())
                .collect();

            self.grid = grid;
        } else {
        }
    }

    pub fn write_note(&mut self, note: Note) {
        self.notes.push(note);
    }

    pub fn set_variable(&mut self, name: char, value: char) {
        self.variables.insert(name, value);
    }

    pub fn read_variable(&self, name: char) -> char {
        *self.variables.get(&name).unwrap_or(&'.')
    }

    pub fn clear_all_variables(&mut self) {
        self.variables = HashMap::new();
    }

    pub fn lock(&mut self, row: i32, col: i32) {
        self.locks.insert((row, col));

        self.ports
            .entry((row, col))
            .or_insert_with(|| format!("Port({},{})", row, col));
    }

    pub fn lock_with_name(&mut self, row: i32, col: i32, name: String) {
        self.locks.insert((row, col));

        self.ports.entry((row, col)).or_insert(name);
    }

    pub fn is_locked(&self, row: i32, col: i32) -> bool {
        self.locks.contains(&(row, col))
    }

    pub fn unlock_all(&mut self) {
        self.locks = HashSet::new();
    }
}
