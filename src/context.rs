use std::collections::HashSet;
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
    pub notes: Vec<MidiNote>,
    pub locks: HashSet<(i32, i32)>,
    pub ticks: usize,
    pub tempo: u64,
    pub divisions: u64,
    pub tick_time: u64,
}

impl Context {
    pub fn new(grid: Vec<Vec<char>>, tempo: u64, divisions: u64) -> Context {
        Context {
            grid,
            notes: Vec::new(),
            locks: HashSet::new(),
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
        if 0 <= row && row < self.grid.len() && 0 <= col && col < self.grid[0].len() {
            self.grid[row][col]
        } else {
            '.'
        }
    }

    pub fn listen(&self, name: &str, row: i32, col: i32, default: char) -> Port {
        let value = self.read(row, col);
        let value = if value == '.' { default } else { value };
        Port::new(name, row, col, value)
    }

    pub fn write(&mut self, row: i32, col: i32, chr: char) {
        let row = row as usize;
        let col = col as usize;
        if 0 <= row && row < self.grid.len() && 0 <= col && col < self.grid[0].len() {
            self.grid[row][col] = chr;
        }
    }

    pub fn write_note(&mut self, note: MidiNote) {
        self.notes.push(note);
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
