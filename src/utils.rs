pub const HELP: &str = "
OPERATORS
[A]dd: Outputs sum of inputs.               [B] subtract: Outputs difference of inputs.
[C]lock: Outputs modulo of frame.           [D]elay: Bangs on modulo of frame.
[E]ast: Moves eastward, or bangs.           [F] if: Bangs if inputs are equal.
[G]enerator: Writes operands with offset.   [H]alt: Halts southward operand.
[I]ncrement: Increments southward operand.  [J]umper: Outputs northward operand.
[K]onkat: Reads multiple variables.         [L]ess: Outputs smallest of inputs.
[M]ultiply: Outputs product of inputs.      [N]orth: Moves Northward, or bangs.
[O] read: Reads operand with offset.        [P]ush: Writes eastward operand.
[Q]uery: Reads operands with offset.        [R]andom: Outputs random value.
[S]outh: Moves southward, or bangs.         [T]rack: Reads eastward operand.
[U]clid: Bangs on Euclidean rhythm.         [V]ariable: Reads and writes variable.
[W]est: Moves westward, or bangs.           [X] write: Writes operand with offset.
[Y] jymper: Outputs westward operand.       [Z] lerp: Transitions operand to input.
[*] bang: Bangs neighboring operands.       [#] comment: Halts a line.
[:] MIDI: Sends a MIDI note.                [;] scaler: Sends degree of a scale as a MIDI note
[>] sampler: Plays a sample                 [~] synth: Plays a built-in synth's note
[{] snippet save: Saves a snippet on bang   [}] snippet load: Loads a snippet on bang
[[] save: Saves to a file on bang           []] load: Loads a file on bang
[@] globals: Global key and scale

CONTROLS
[`]: select mode      [/]: move mode
[=/-]: tempo up/down  [CTRL-c]: copy selected cells
[CTRL-v]: paste       [CTRL-d]: clear the grid
[CTRL-h]: help        [CTRL-p]: change midi port
";

pub const NATURAL_NOTES: [u8; 7] = [9, 11, 0, 2, 4, 5, 7];
pub const SHARP_NOTES: [u8; 7] = [10, 12, 1, 3, 5, 6, 8];
pub const SCALES: [[u8; 7]; 26] = [
    //major
    [0, 2, 4, 5, 7, 9, 11],
    //minor
    [0, 2, 3, 5, 7, 8, 10],
    //dorian
    [0, 2, 3, 5, 7, 9, 10],
    //phrygian
    [0, 1, 3, 5, 7, 8, 10],
    //lydian
    [0, 2, 4, 6, 7, 9, 11],
    //mixolydian
    [0, 2, 4, 5, 7, 9, 10],
    //locrian
    [0, 1, 3, 5, 6, 8, 10],
    //harmonicMin
    [0, 2, 3, 5, 7, 8, 11],
    //harmonicMaj
    [0, 2, 4, 5, 7, 8, 11],
    //melodicMin
    [0, 2, 3, 5, 7, 9, 11],
    //melodicMaj
    [0, 2, 4, 5, 7, 8, 10],
    //superLocrian
    [0, 1, 3, 4, 6, 8, 10],
    //romanianMinor
    [0, 2, 3, 6, 7, 9, 10],
    //hungarianMinor
    [0, 2, 3, 6, 7, 8, 11],
    //neapolitanMinor
    [0, 1, 3, 5, 7, 8, 11],
    //enigmatic
    [0, 1, 4, 6, 8, 10, 11],
    //spanish
    [0, 1, 4, 5, 7, 8, 10],
    //leadingWhole
    [0, 2, 4, 6, 8, 10, 11],
    //lydianMinor
    [0, 2, 4, 6, 7, 8, 10],
    //neapolitanMajor
    [0, 1, 3, 5, 7, 9, 11],
    //locrianMajor
    [0, 2, 4, 5, 6, 8, 10],
    //todi
    [0, 1, 3, 6, 7, 8, 11],
    //purvi
    [0, 1, 4, 6, 7, 8, 11],
    //marva
    [0, 1, 4, 6, 7, 9, 11],
    //bhairav
    [0, 1, 4, 5, 7, 8, 11],
    //ahirbhairav
    [0, 1, 4, 5, 7, 9, 10],
];

pub fn get_scale_name(value: char) -> Option<&'static str> {
    match value {
        '0' => Some("Major"),
        '1' => Some("Minor"),
        '2' => Some("Dorian"),
        '3' => Some("Phrygian"),
        '4' => Some("Lydian"),
        '5' => Some("Mixolydian"),
        '6' => Some("Locrian"),
        '7' => Some("Harmonic Minor"),
        '8' => Some("Harmonic Major"),
        '9' => Some("Melodic Minor"),
        'a' => Some("Melodic Major"),
        'b' => Some("Superlocrian"),
        'c' => Some("Romanian Minor"),
        'd' => Some("Hungarian Minor"),
        'e' => Some("Neapolitan Minor"),
        'f' => Some("Enigmatic"),
        'g' => Some("Spanish"),
        'h' => Some("Leading Whole"),
        'i' => Some("Lydian Minor"),
        'j' => Some("Neapolitan Major"),
        'k' => Some("Locrian Major"),
        'l' => Some("Todi"),
        'm' => Some("Purvi"),
        'n' => Some("Marva"),
        'o' => Some("Bhairav"),
        'p' => Some("Ahirbhairav"),
        _ => Some("Major"),
    }
}

pub fn get_key_name(value: char) -> Option<&'static str> {
    match value {
        'C' => Some("C"),
        'D' => Some("D"),
        'E' => Some("E"),
        'F' => Some("F"),
        'G' => Some("G"),
        'A' => Some("A"),
        'B' => Some("B"),
        'a' => Some("A#"),
        'c' => Some("C#"),
        'd' => Some("D#"),
        'f' => Some("F#"),
        'g' => Some("G#"),
        _ => Some("C"),
    }
}
