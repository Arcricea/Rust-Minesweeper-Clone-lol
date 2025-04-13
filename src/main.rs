use crate::graphics_and_window::run;
use glam::Vec2;
pub mod camera;
pub mod game_objects;
pub mod graphics_and_window;
pub mod input_handling;
pub mod instance;
pub mod texture;

const CAMERA_MOVE_SPEED: f32 = 0.000000001;
const BOARD_WIDTH: u32 = 15;
const BOARD_LENGTH: u32 = 15;
const MINE_COUNT: u32 = 30;

trait MineActiveTrait {
    fn is_active(&self) -> bool;
    fn set_active(&mut self, active: bool);
}

pub struct CommonMineState {
    active: bool,
    mine_index: u16,
}

impl MineActiveTrait for CommonMineState {
    fn is_active(&self) -> bool {
        self.active
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

pub enum Mines {
    Default(CommonMineState),
}

impl Mines {
    pub fn activate(&mut self) {
        match self {
            Mines::Default(state) => state.set_active(true),
        }
    }

    fn is_active(&self) -> bool {
        match self {
            Mines::Default(state) => state.is_active(),
        }
    }

    fn get_index(&self) -> u16 {
        match self {
            Mines::Default(state) => state.mine_index,
        }
    }
}
pub struct Tiles {
    board_position: Vec2,
    position: Vec2,
    size: f32,
    mine: Option<Mines>,
    pub clicked: bool,
    pub flagged: bool,
}

impl Tiles {
    pub fn new(board_position: Vec2, position: Vec2, mine: Option<Mines>, size: f32) -> Tiles {
        Self {
            board_position,
            position,
            mine,
            size,
            clicked: false,
            flagged: false,
        }
    }

    pub fn is_clicked(&self, mouse_pos: Vec2) -> bool {
        if self.position.x <= mouse_pos.x
            && self.position.x > mouse_pos.x - self.size
            && self.position.y <= mouse_pos.y
            && self.position.y > mouse_pos.y - self.size
        {
            return true;
        }
        false
    }

    pub fn has_mine(&self) -> bool {
        self.mine.is_some()
    }

    pub fn set_mine(&mut self, mine: Option<Mines>) {
        self.mine = mine;
    }

    pub fn get_mine_index(&self) -> Option<u16> {
        self.mine.as_ref().map(|mine| mine.get_index())
    }
}

pub struct GameState {
    board: Vec<Tiles>,
}

impl GameState {
    pub fn new(board: Vec<Tiles>) -> GameState {
        Self { board }
    }
}

fn main() {
    pollster::block_on(run());
}
