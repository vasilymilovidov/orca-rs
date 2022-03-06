use std::collections::HashMap;
use rand::Rng;
use crate::context::Context;
use crate::midi::MidiNote;

pub fn char_to_base_36(c: char) -> Option<(u8, bool)> {
    if c >= '0' && c <= '9' {
        Some((c as u8 - '0' as u8, false))
    } else if c >= 'a' && c <= 'z' {
        Some((c as u8 + 10 - 'a' as u8, false))
    } else if c >= 'A' && c <= 'Z' {
        Some((c as u8 + 10 - 'A' as u8, true))
    } else {
        None
    }
}

pub fn base_36_to_char(c: u8, upper: bool) -> char {
    let c = c % 36;
    let c = if c < 10 {
        c as u8 + '0' as u8
    } else if upper {
        c as u8 - 10 + 'A' as u8
    } else {
        c as u8 - 10 + 'a' as u8
    };
    c as char
}


pub fn add(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((a, a_upper)), Some((b, b_upper))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        context.grid[row + 1][col] = base_36_to_char(a + b, a_upper || b_upper);
    }
}


pub fn sub(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((a, a_upper)), Some((b, b_upper))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        let diff = if a > b { a - b } else { b - a };
        context.grid[row + 1][col] = base_36_to_char(diff, a_upper || b_upper);
    }
}

pub fn delay(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((rate, _)), Some((delay_mod, _))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        if context.ticks % (rate as usize * delay_mod as usize) == 0 {
            context.grid[row + 1][col] = '*';
        }
    }
}

pub fn random(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((min, min_upper)), Some((max, max_upper))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        let mut rng = rand::thread_rng();
        let r = rng.gen_range(min..max);
        let c = base_36_to_char(r, min_upper || max_upper);
        context.grid[row + 1][col] = c;
    }
}

pub fn midi_note(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if grid[row - 1][col] == '*' || grid[row][col - 1] == '*' || grid[row + 1][col] == '*' {
        if let (
            Some((channel, _)),
            Some((octave, _)),
            Some((note, upper)),
            Some((velocity, _)),
            Some((duration, _))
        ) = (
            char_to_base_36(grid[row][col + 1]),
            char_to_base_36(grid[row][col + 2]),
            char_to_base_36(grid[row][col + 3]),
            char_to_base_36(grid[row][col + 4]),
            char_to_base_36(grid[row][col + 5]),
        ) {
            if note >= 10 {
                let midi_note = MidiNote::from_base_36(
                    channel, octave, note, !upper, velocity, duration,
                    context.tick_time,
                );
                context.notes.push(midi_note);
            }
        }
    }
}

pub fn clock(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((rate, _)), Some((clock_mod, mod_upper))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        let value = context.ticks / rate as usize % clock_mod as usize;
        context.grid[row + 1][col] = base_36_to_char(value as u8, mod_upper);
    }
}

pub fn track(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((key, _)), Some((len, _))) = (
        char_to_base_36(grid[row][col - 2]),
        char_to_base_36(grid[row][col - 1])
    ) {
        context.grid[row + 1][col] = grid[row][col + 1 + (key % len) as usize];
    }
}

pub fn east(context: &mut Context, row: usize, col: usize) {
    context.grid[row][col + 1] = context.grid[row][col];
    context.grid[row][col] = '.';
}

pub fn west(context: &mut Context, row: usize, col: usize) {
    context.grid[row][col - 1] = context.grid[row][col];
    context.grid[row][col] = '.';
}

pub fn north(context: &mut Context, row: usize, col: usize) {
    context.grid[row - 1][col] = context.grid[row][col];
    context.grid[row][col] = '.';
}

pub fn south(context: &mut Context, row: usize, col: usize) {
    context.grid[row + 1][col] = context.grid[row][col];
    context.grid[row][col] = '.';
}

pub fn condition(context: &mut Context, row: usize, col: usize) {
    if context.grid[row][col - 1] == context.grid[row][col + 1] && context.grid[row][col - 1] != '.' {
        context.grid[row + 1][col] = '*';
    } else {
        context.grid[row + 1][col] = '.';
    }
}

pub fn increment(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((step, _)), Some((increment_mod, mod_upper))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        let value = if let Some((value, _)) = char_to_base_36(grid[row + 1][col]) {
            (value + step) % increment_mod
        } else {
            0
        };
        context.grid[row + 1][col] = base_36_to_char(value, mod_upper);
    }
}

pub fn jump(context: &mut Context, row: usize, col: usize) {
    context.grid[row + 1][col] = context.grid[row - 1][col];
}

pub fn jymp(context: &mut Context, row: usize, col: usize) {
    context.grid[row][col + 1] = context.grid[row][col - 1];
}

pub fn lesser(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((a, a_upper)), Some((b, b_upper))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        let less = if a < b { a } else { b };
        context.grid[row + 1][col] = base_36_to_char(less, a_upper || b_upper);
    }
}

pub fn multiply(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((a, a_upper)), Some((b, b_upper))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        context.grid[row + 1][col] = base_36_to_char(a * b, a_upper || b_upper);
    }
}

pub fn read(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((x, _)), Some((y, _))) = (
        char_to_base_36(grid[row][col - 2]),
        char_to_base_36(grid[row][col - 1])
    ) {
        context.grid[row + 1][col] = context.grid[row + y as usize][col + 1 + x as usize];
    }
}

pub fn push(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((key, _)), Some((len, _))) = (
        char_to_base_36(grid[row][col - 2]),
        char_to_base_36(grid[row][col - 1])
    ) {
        context.grid[row + 1][col + (key % len) as usize] = grid[row][col + 1];
    }
}

pub fn query(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((x, _)), Some((y, _)), Some((len, _))) = (
        char_to_base_36(grid[row][col - 3]),
        char_to_base_36(grid[row][col - 2]),
        char_to_base_36(grid[row][col - 1])
    ) {
        for i in 0..len {
            context.grid[row + 1][col - (len - i) as usize + 1] = context.grid[row + y as usize][col + 1 + (x + i) as usize];
        }
    }
}

pub fn generate(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((x, _)), Some((y, _)), Some((len, _))) = (
        char_to_base_36(grid[row][col - 3]),
        char_to_base_36(grid[row][col - 2]),
        char_to_base_36(grid[row][col - 1])
    ) {
        for i in 0..len {
            context.grid[row + 1 + y as usize][col + (x + i) as usize] = context.grid[row][col + 1 + i as usize];
        }
    }
}

pub fn write(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((x, _)), Some((y, _))) = (
        char_to_base_36(grid[row][col - 2]),
        char_to_base_36(grid[row][col - 1])
    ) {
        context.grid[row + 1 + y as usize][col + x as usize] = context.grid[row][col + 1];
    }
}

pub fn interpolate(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((rate, _)), Some((target, target_upper))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        let value = if let Some((value, _)) = char_to_base_36(grid[row + 1][col]) {
            (value + rate).min(target)
        } else {
            0
        };
        context.grid[row + 1][col] = base_36_to_char(value, target_upper);
    }
}

pub fn euclid(context: &mut Context, row: usize, col: usize) {
    let grid = &context.grid;
    if let (Some((step, _)), Some((max, _))) = (
        char_to_base_36(grid[row][col - 1]),
        char_to_base_36(grid[row][col + 1])
    ) {
        if (step as usize * (context.ticks + max as usize - 1) % max as usize) as u8 + step >= max {
            context.grid[row + 1][col] = '*';
        }
    }
}

pub fn get_tick_operators() -> HashMap<char, fn(&mut Context, usize, usize)> {
    let mut operators: HashMap<char, fn(&mut Context, usize, usize)> = HashMap::new();
    operators.insert('A', add);
    operators.insert('B', sub);
    operators.insert('C', clock);
    operators.insert('D', delay);
    operators.insert('E', east);
    operators.insert('F', condition);
    operators.insert('G', generate);
    // operators.insert('H', halt);
    operators.insert('I', increment);
    operators.insert('J', jump);
    // operators.insert('K', concat);
    operators.insert('L', lesser);
    operators.insert('M', multiply);
    operators.insert('N', north);
    operators.insert('O', read);
    operators.insert('P', push);
    operators.insert('Q', query);
    operators.insert('R', random);
    operators.insert('S', south);
    operators.insert('T', track);
    operators.insert('U', euclid);
    // operators.insert('V', variable);
    operators.insert('W', west);
    operators.insert('X', write);
    operators.insert('Y', jymp);
    operators.insert('Z', interpolate);
    operators
}

pub fn get_bang_operators() -> HashMap<char, fn(&mut Context, usize, usize)> {
    let mut operators: HashMap<char, fn(&mut Context, usize, usize)> = HashMap::new();
    for (c, operator) in get_tick_operators() {
        operators.insert(c.to_ascii_lowercase(), operator);
    }
    operators.insert(':', midi_note);
    operators
}

pub fn grid_tick(
    context: &mut Context,
    tick_operators: &HashMap<char, fn(&mut Context, usize, usize)>,
    bang_operators: &HashMap<char, fn(&mut Context, usize, usize)>,
) {
    let rows = context.grid.len();
    let cols = context.grid[0].len();

    // clear previous bangs
    for row in 0..rows {
        for col in 0..cols {
            if context.grid[row][col] == '*' {
                context.grid[row][col] = '.';
            }
        }
    }

    // apply grid operators (which may produce new bangs)
    for row in 0..rows {
        for col in 0..cols {
            if let Some(operator) = tick_operators.get(&context.grid[row][col]) {
                operator(context, row, col);
            }
        }
    }

    // apply bang operators on current bangs
    for row in 0..rows {
        for col in 0..cols {
            if let Some(operator) = bang_operators.get(&context.grid[row][col]) {
                if context.grid[row - 1][col] == '*'
                    || context.grid[row][col - 1] == '*'
                    || context.grid[row + 1][col] == '*' {
                    operator(context, row, col);
                }
            }
        }
    }

    context.ticks += 1;
}
