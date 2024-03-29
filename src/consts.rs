pub mod game {
    pub const PLAYER_SPEED: f32 = 5.0;
    pub const PLAYER_SLOWMO_FACTOR: f32 = 2.2;
    pub const COLLECTOR_EDGE_LENGTH: f32 = 80.0;
    pub const FIELD_EDGE_LENGTH: f32 = 500.0;
    const OBSTACLE_WARNING_DRAW_TIME: u32 = 20;
    pub const OBSTACLE_WARNING_FINISH_WAIT_TIME: u32 = 20;
    pub const OBSTACLE_PRE_SPAWN_WARN_TIME: u32 =
        OBSTACLE_WARNING_DRAW_TIME + OBSTACLE_WARNING_FINISH_WAIT_TIME;
    pub const OBSTACLE_HIDE_DELAY: u32 = 20;
    pub const OBSTACLE_WARNING_MOVE_SPEED: f32 =
        FIELD_EDGE_LENGTH / OBSTACLE_WARNING_DRAW_TIME as f32;
    pub const SPAWN_RATE_FACTOR: f32 = 6.;
    pub const SPAWN_RATE_SUBTRACT: f32 = 1.2;
}

pub mod system {
    pub const WIN_WIDTH: u32 = 800;
    pub const WIN_HEIGHT: u32 = 600;
    pub const FPS_GRAPH_SAMPLE_COUNT: usize = 64;

}

pub mod graphics {
    pub const FONT_NAME: &str = "Georgia.ttf";
    pub const FONT_SIZE_PT: f32 = 18.0;

    pub const HUD_CORNER_PADDING: f32 = 15.0;

    pub const FIELD_EDGE_BORDER_WIDTH: f32 = 1.0;
    pub const OBSTACLE_WARNING_WIDTH: f32 = 1.0;
}
