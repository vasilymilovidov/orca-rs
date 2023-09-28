## Orca-rs

***Work in progress***

My variation on the original repo's port of hundredrabbits' Orca.
It has a new TUI made with ```ratatui```. Basic functionality such as copying/pasting, changing tempo, midi ports, etc., has been added.
Additionally, there are a few non-standard features for Orca, such as global key and scale, operators for playing samples and synths, and a somewhat convoluted way of saving and loading sessions and snippets.
The project started primarily as a way for me to learn Rust, so many parts of the code are in need of improved implementation and rewriting, but I'll get there. 

![orca](https://github.com/vasilymilovidov/orca-rs/blob/main/Orca.png?raw=true)

### Building
```
cargo build --release
```
Not properly tested on Linux and Windows, but it should build.

### Usage
```
OPERATORS
[A]dd: Outputs sum of inputs.               [B] subtract(a b): Outputs difference of inputs.
[C]lock: Outputs modulo of frame.           [D]elay(rate mod): Bangs on modulo of frame.
[E]ast: Moves eastward, or bangs.           [F] if(a b): Bangs if inputs are equal.
[G]enerator: Writes operands with offset.   [H]alt: Halts southward operand.
[I]ncrement: Increments southward operand.  [J]umper(val): Outputs northward operand.
[K]onkat: Reads multiple variables.         [L]ess(a b): Outputs smallest of inputs.
[M]ultiply: Outputs product of inputs.      [N]orth: Moves Northward, or bangs.
[O] read: Reads operand with offset.        [P]ush(len key val): Writes eastward operand.
[Q]uery: Reads operands with offset.        [R]andom(min max): Outputs random value.
[S]outh: Moves southward, or bangs.         [T]rack(key len val): Reads eastward operand.
[U]clid: Bangs on Euclidean rhythm.         [V]ariable(write read): Reads and writes variable.
[W]est: Moves westward, or bangs.           [X] write(x y val): Writes operand with offset.
[Y] jymper: Outputs westward operand.       [Z] lerp(rate target): Transitions operand to input.
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
```

#### Save/Load operators - `[` and `]` for files, `{` and `}` for snippets
Saving/loading is implemented as a pair of operators: write the name of a file you want to save/load, and send a bang to the operator.
The same goes for snippet saving and loading. After you load a snippet, you can paste it into the grid.

#### Scaler operator - `;`
Similar to the MIDI operator, but sends MIDI notes based on the degree of a scale. The scale is defined by the global key and scale.

#### Globals operator - `@`
An operator that allows you to set global key and scale.

#### Synth and Sampler - `~` and `>`
A very basic integration of `fundsp` crate. Primarily for testing purposes.
The sampler operator plays samples from the `orca/samples` located in your root. If it's empty, it generates noise. You can pitch samples up and down, but only in a rudimentary manner.
The synth operator has 4 simple engines: `sine`, `square`, `saw`, `triangle`, and  `kick`. The waveforms engines have an `fm` parameter. The same slot controls the noise level on `kick`.

#### Arguments
'orca-rs last' opens the last session that was closed. Args 2 and 3 are for specifying number of rows and columns. 




