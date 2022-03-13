use std::collections::{HashMap, HashSet};
use crate::midi::MidiNote;


#[derive(Clone)]
#[derive(Debug)]
pub struct Port {
    pub name: String,
    pub row: i32,
    pub col: i32,
    pub value: char,
}

impl Port {
    pub fn new(name: &str, row: i32, col: i32, value: char) -> Port {
        Port { name: String::from(name), row, col, value }
    }
}


pub struct Context {
    pub grid: Vec<Vec<char>>,
    pub width: usize,
    pub height: usize,
    pub notes: Vec<MidiNote>,
    pub locks: HashSet<(i32, i32)>,
    pub variables: HashMap<char, char>,
    pub ticks: usize,
    pub tempo: u64,
    pub divisions: u64,
    pub tick_time: u64,
}

impl Context {
    pub fn new(grid: Vec<Vec<char>>, tempo: u64, divisions: u64) -> Context {
        let width = grid[0].len();
        let height = grid.len();
        Context {
            grid,
            width,
            height,
            notes: Vec::new(),
            locks: HashSet::new(),
            variables: HashMap::new(),
            ticks: 0,
            tempo,
            divisions,
            tick_time: 60000 / (tempo * divisions),
        }
    }
    #[allow(dead_code)]
    pub fn display(&self) {
        let rows = self.grid.len();
        let cols = self.grid[0].len();
        for row in 0..rows {
            for col in 0..cols {
                print!("{}", self.grid[row][col]);
            }
            println!();
        }
        println!("{:?}", self.notes);
    }

    pub fn read(&self, row: i32, col: i32) -> char {
        let row = row as usize;
        let col = col as usize;
        if 0 <= row && row < self.height && 0 <= col && col < self.width {
            self.grid[row][col]
        } else {
            '\0'
        }
    }

    pub fn listen(&self, name: &str, row: i32, col: i32, default: char) -> Port {
        let value = self.read(row, col);
        let value = if value == '\0' { default } else { value };
        Port::new(name, row, col, value)
    }

    pub fn write(&mut self, row: i32, col: i32, value: char) {
        let row = row as usize;
        let col = col as usize;
        if 0 <= row && row < self.height && 0 <= col && col < self.width {
            self.grid[row][col] = value;
        }
    }

    pub fn write_note(&mut self, note: MidiNote) {
        self.notes.push(note);
    }

    pub fn set_variable(&mut self, name: char, value: char) {
        self.variables.insert(name, value);
    }

    pub fn read_variable(&self, name: char) -> char {
        *self.variables.get(&name).unwrap_or(&'\0')
    }

    pub fn clear_all_variables(&mut self) {
        self.variables = HashMap::new();
    }

    pub fn lock(&mut self, row: i32, col: i32) {
        self.locks.insert((row, col));
    }

    pub fn is_locked(&self, row: i32, col: i32) -> bool {
        self.locks.contains(&(row, col))
    }

    pub fn unlock_all(&mut self) {
        self.locks = HashSet::new();
    }
}
