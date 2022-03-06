use crate::midi::MidiNote;

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
}
