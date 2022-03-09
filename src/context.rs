use crate::midi::MidiNote;


#[derive(Clone)]
pub struct Port {
    pub name: String,
    pub row: i32,
    pub col: i32,
    pub value: char
}

impl Port {
    pub fn new(name: &str, row: i32, col: i32, value: char) -> Port {
        Port { name: String::from(name), row, col, value }
    }
}


pub struct Context {
    pub grid: Vec<Vec<char>>,
    pub notes: Vec<MidiNote>,
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
        self.grid[row as usize][col as usize]
    }

    pub fn listen(&self, name: &str, row: i32, col: i32, default: char) -> Port {
        let value = self.read(row, col);
        let value = if value == '.' { default } else { value };
        Port::new(name, row, col, value)
    }

    pub fn write(&mut self, row: i32, col: i32, chr: char) {
        self.grid[row as usize][col as usize] = chr;
    }

    pub fn write_note(&mut self, note: MidiNote) {
        self.notes.push(note);
    }
}
