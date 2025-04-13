Using winit/wgpu. 

# Current progress:
- Minesweeper with Chording/Flags
- Camera movement with up/down/left/right, Zoom out with mouse wheel
- "Restart" via Tab to clear the instance buffer and delete the board, and then click Spacebar to make a new board / fill the instance buffer

## Installation
Install rust using rustup if you don't have it - https://www.rust-lang.org/tools/install and follow the tutorial
```
git clone https://github.com/Arcricea/Rust-Minesweeper-Clone-lol
cd Rust-Minesweeper-Clone-lol
cargo run
```
I will set up cargo build later with build.rs for sprite copying

## To do:
### GRAPHICS
- ~~Non-square screen sizes makes it explode :c~~
- ~~Camera Zoom is on the left, rather than in the middle~~

### UI
- Add UI / Static Elements
- Restart button
- Change game size while ingame
- Win/Loss screen

### SOUND
- Add Music
- Add SFX (?)

### GAMEPLAY 
- You can start with a mine? This is so sad.

### MISC
- build.rs 
