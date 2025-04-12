use crate::instance::{Instance, InstanceRaw};
use crate::{graphics_and_window::tex_from_coords, BOARD_LENGTH, BOARD_WIDTH};
use crate::{GameState, Mines, Tiles, MINE_COUNT};
use glam::{Vec2, Vec4};
use std::collections::HashMap;

pub const Z_BOARD: f32 = 0.01;
pub const Z_MINE: f32 = 0.02;

pub fn create_hashmap() -> HashMap<String, Vec4> {
    let mut sprites = HashMap::new();
    sprites.insert(String::from("Teto"), tex_from_coords([28, 0, 32, 4]));
    sprites.insert(String::from("Square"), tex_from_coords([0, 0, 1, 1]));
    sprites.insert(String::from("Circle"), tex_from_coords([1, 0, 2, 1]));

    // game objects
    sprites.insert(String::from("Mines"), tex_from_coords([0, 1, 1, 2]));
    sprites.insert(String::from("Tiles"), tex_from_coords([0, 2, 1, 3]));
    sprites.insert(String::from("Flags"), tex_from_coords([2, 1, 3, 2]));

    // number tiles
    sprites.insert(String::from("1"), tex_from_coords([2, 0, 3, 1]));
    sprites.insert(String::from("2"), tex_from_coords([3, 0, 4, 1]));
    sprites.insert(String::from("3"), tex_from_coords([4, 0, 5, 1]));
    sprites.insert(String::from("4"), tex_from_coords([5, 0, 6, 1]));
    sprites.insert(String::from("5"), tex_from_coords([6, 0, 7, 1]));
    sprites.insert(String::from("6"), tex_from_coords([7, 0, 8, 1]));
    sprites.insert(String::from("7"), tex_from_coords([8, 0, 9, 1]));
    sprites.insert(String::from("8"), tex_from_coords([9, 0, 10, 1]));
    sprites.insert(String::from("0"), tex_from_coords([0, 0, 1, 1]));

    // return
    sprites
}

pub fn create_minefield(sprites: HashMap<String, Vec4>) -> (Vec<InstanceRaw>, GameState) {
    // initialize
    let mut objects: Vec<InstanceRaw> = Vec::new();
    let mut board: Vec<Tiles> = Vec::with_capacity((BOARD_WIDTH * BOARD_LENGTH).into());
    let mut mine_size: f32 = 1.0 / BOARD_LENGTH as f32;
    if BOARD_WIDTH > BOARD_LENGTH {
        mine_size = 1.0 / BOARD_WIDTH as f32;
    }
    // Tiles

    for row in 0..BOARD_LENGTH {
        for col in 0..BOARD_WIDTH {
            objects.push(Instance::to_raw(
                Vec2::new(
                    (col as f32 + 0.5) * mine_size,
                    (row as f32 + 0.5) * mine_size,
                ),
                0.0,
                Vec2::new(mine_size, mine_size),
                Z_BOARD,
                *sprites.get("Tiles").expect("No Tiles :c"),
                0,
            ));
            board.push(Tiles::new(
                Vec2::new(col as f32, row as f32),
                Vec2::new(col as f32 * mine_size, row as f32 * mine_size),
                None,
                mine_size,
            ));
        }
    }
    // Mines

    let mut mine_count = 0;
    while mine_count < MINE_COUNT {
        let index = rand::random_range(0..((BOARD_LENGTH * BOARD_WIDTH) - 1)) as usize;
        if !board[index].has_mine() {
            board[index].set_mine(Some(Mines::Default(crate::CommonMineState {
                active: false,
                mine_index: 1 + mine_count as u16,
            })));
            mine_count += 1;
        }
    }
    // return
    (objects, GameState::new(board))
}
