extern crate quicksilver;
extern crate rand;

mod consts;
mod error;
mod graphics;
mod util;

use quicksilver::{
    geom::{Rectangle, Shape, Vector},
    graphics::{Background, Color, Font, FontStyle},
    input::{Key, Keyboard},
    lifecycle::{run, Asset, Settings, State, Window},
};

use rand::{rngs::ThreadRng, Rng};

use std::{
    cmp, time::{Duration, Instant},
};

use consts::{game::*, graphics::*, system::*};
use error::{Error, Result};
use graphics::Strobe;
use util::{Countdown, FpsGraph};
use core::borrow::Borrow;

#[derive(Debug)]
enum Direction {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy)]
struct Obstacle {
    /// A measurement of where the obstacle is coming from. 1 rixel = 1 pixel around the perimeter
    /// of the playfield, starting at the upper-left corner.
    rixel: f32,
    speed: f32,
    width: f32,
    length: f32,
    lifetime: f32,
}

impl Obstacle {
    /// Randomly generate a new obstacle.
    fn spawn(rng: &mut ThreadRng) -> Obstacle {
        let width = rng.gen_range(6.0, 14.0);
        let rixel = FIELD_EDGE_LENGTH * rng.gen_range(0, 4) as f32;
        let rixel = rixel + rng.gen_range(width / 2., FIELD_EDGE_LENGTH - width / 2.);
        Obstacle {
            rixel: rixel,
            speed: 3.0,
            width: width,
            length: 300.0,
            lifetime: -(OBSTACLE_PRE_SPAWN_WARN_TIME as f32),
        }
    }

    /// Calculates the distance in rixels from the given rixel to the next corner.
    fn rixels_to_next_corner(rixel: f32) -> f32 {
        FIELD_EDGE_LENGTH - (rixel % FIELD_EDGE_LENGTH)
    }

    /// Convert a numerical position (in rixels) to a side of the screen.
    fn rixel_to_direction(rixel: f32) -> Result<Direction> {
        if rixel > 0. && rixel < FIELD_EDGE_LENGTH {
            Ok(Direction::North)
        } else if rixel < FIELD_EDGE_LENGTH * 2. {
            Ok(Direction::East)
        } else if rixel < FIELD_EDGE_LENGTH * 3. {
            Ok(Direction::South)
        } else if rixel < FIELD_EDGE_LENGTH * 4. {
            Ok(Direction::West)
        } else {
            Err(Error::ObstacleRixelOutOfBounds(rixel))
        }
    }

    /// Convert obstacle positioning data (rixel, distance from edge, and dimensions) to a
    /// rectangle.
    fn positioning_to_rectangle(
        rixel: f32,
        distance: f32,
        length: f32,
        width: f32,
    ) -> Result<Rectangle> {
        use Direction::*;

        let dir = Self::rixel_to_direction(rixel)?;
        let distance_back = FIELD_EDGE_LENGTH - Self::rixels_to_next_corner(rixel);
        Ok(Rectangle::new(
            // Position
            match dir {
                North => (rixel - width / 2., -length + distance),
                East => (
                    FIELD_EDGE_LENGTH - distance,
                    rixel - FIELD_EDGE_LENGTH - width / 2.,
                ),
                South => (
                    rixel - distance_back * 2. - FIELD_EDGE_LENGTH - width / 2.,
                    FIELD_EDGE_LENGTH - distance,
                ),
                West => (
                    -length + distance,
                    FIELD_EDGE_LENGTH * 4. - rixel - width / 2.,
                ),
            },
            // Dimensions
            match dir {
                North | South => (width, length),
                East | West => (length, width),
            },
        ))
    }

    /// Get this obstacle's rectangle.
    fn rectangle(&self) -> Rectangle {
        let distance = if self.lifetime * self.speed > FIELD_EDGE_LENGTH {
            FIELD_EDGE_LENGTH
        } else {
            self.lifetime * self.speed
        };

        let length = if self.lifetime < 0. || self.lifetime > self.total_lifetime() {
            0.
        } else if self.lifetime * self.speed < self.length {
            self.lifetime * self.speed
        } else if self.lifetime * self.speed > FIELD_EDGE_LENGTH {
            self.length - (self.lifetime * self.speed - FIELD_EDGE_LENGTH)
        } else {
            self.length
        };

        Obstacle::positioning_to_rectangle(self.rixel, distance, length, self.width).unwrap()
    }

    /// Get the rixel on the opposite side of the perimeter.
    fn opposite(&self) -> f32 {
        let to_next_corner = FIELD_EDGE_LENGTH - (self.rixel % FIELD_EDGE_LENGTH);
        (self.rixel + to_next_corner + FIELD_EDGE_LENGTH + to_next_corner)
            % (FIELD_EDGE_LENGTH * 4.)
    }

    /// The lifetime value at which this obstacle has moved completely offscreen.
    fn total_lifetime(&self) -> f32 {
        (FIELD_EDGE_LENGTH + self.length) / self.speed
    }
}

/// Tracks information about the player and their avatar.
#[derive(Debug)]
struct Player {
    rect: Rectangle,
    score: u32,
    color: Color,
}

impl Player {
    fn new() -> Player {
        Player {
            rect: Rectangle::new((0, 0), (50, 50)),
            score: 0,
            color: Color::RED,
        }
    }

    fn collector_rectangle(&self) -> Rectangle {
        Rectangle::new_sized((COLLECTOR_EDGE_LENGTH, COLLECTOR_EDGE_LENGTH))
            .with_center(self.rect.center())
    }
}

struct GameState {
    obstacles: Vec<Obstacle>,
    player: Player,
    rng: ThreadRng,

    last_spawned: Option<Instant>,
    spawn_interval: Duration,

    is_running: bool,
    reset_countdown: Option<Countdown>,

    fps_graph: FpsGraph,
    fps_update_time: Option<Instant>,

    font: Asset<Font>,
    font_style: FontStyle,
}

impl GameState {
    /// Given the player's current score value, decide how long the wait for the next obstacle to
    /// spawn should be.
    fn obstacle_spawn_interval(score: u32) -> Duration {
        let score = cmp::max(100, score);
        let spawntime = ((SPAWN_RATE_FACTOR / (score as f32 / 100.).powf(1. / 3.)
            - SPAWN_RATE_SUBTRACT)
            * 1000.) as u64;
        Duration::from_millis(spawntime)
    }
}

// Drawing logic.
impl GameState {
    fn draw_obstacles(&self, window: &mut Window) -> Result<()> {
        // Draw the obstacle warnings.
        for obstacle in &self.obstacles {
            // Didn't realize Quicksilver had a Line type lol.
            let line_rect = if obstacle.lifetime < 0. {
                let dist = FIELD_EDGE_LENGTH.min(
                    OBSTACLE_WARNING_MOVE_SPEED
                        * (obstacle.lifetime + OBSTACLE_PRE_SPAWN_WARN_TIME as f32),
                );
                Obstacle::positioning_to_rectangle(
                    obstacle.rixel,
                    dist,
                    dist,
                    OBSTACLE_WARNING_WIDTH,
                )
            } else if obstacle.lifetime - obstacle.total_lifetime() < OBSTACLE_HIDE_DELAY as f32 {
                Obstacle::positioning_to_rectangle(
                    obstacle.rixel,
                    FIELD_EDGE_LENGTH,
                    FIELD_EDGE_LENGTH,
                    OBSTACLE_WARNING_WIDTH,
                )
            } else {
                let dist = FIELD_EDGE_LENGTH
                    - ((obstacle.lifetime
                        - OBSTACLE_HIDE_DELAY as f32
                        - obstacle.total_lifetime())
                        * OBSTACLE_WARNING_MOVE_SPEED)
                        .max(0.);
                Obstacle::positioning_to_rectangle(
                    obstacle.opposite(),
                    dist,
                    dist,
                    OBSTACLE_WARNING_WIDTH,
                )
            }?;

            window.draw(&line_rect.on_playfield(), Background::Col(Color::WHITE));
        }

        // Then draw the obstacles themselves.
        for obstacle in &self.obstacles {
            let color = if obstacle.rectangle().overlaps_rectangle(&self.player.rect) && self.reset_countdown.is_some() {
                let countdown = self.reset_countdown.as_ref().unwrap().elapsed();
                Color::RED.strobe(&countdown, Duration::from_millis(500))
            } else {
                Color::RED
            };
            window.draw(
                &obstacle.rectangle().on_playfield(),
                Background::Col(color)
            );
        }

        Ok(())
    }

    fn draw_field_border(&self, window: &mut Window) -> Result<()> {
        window.draw(
            &Rectangle::new(
                (-FIELD_EDGE_BORDER_WIDTH, -FIELD_EDGE_BORDER_WIDTH),
                (
                    FIELD_EDGE_BORDER_WIDTH * 2. + FIELD_EDGE_LENGTH,
                    FIELD_EDGE_BORDER_WIDTH * 2. + FIELD_EDGE_LENGTH,
                ),
            )
            .on_playfield(),
            Background::Col(Color::WHITE),
        );

        window.draw(
            &Rectangle::new((0, 0), (FIELD_EDGE_LENGTH, FIELD_EDGE_LENGTH)).on_playfield(),
            Background::Col(Color::BLACK),
        );

        Ok(())
    }

    fn draw_hud(&mut self, window: &mut Window) -> Result<()> {
        let style = &self.font_style;
        if let Some(fps) = &self.fps_graph.recent_average_fps() {
            self.font.execute(|font| {
                let img = font.render(&format!("{:.0}", fps), style)?;
                window.draw(
                    &Rectangle::new((HUD_CORNER_PADDING, HUD_CORNER_PADDING), img.area().size()),
                    Background::Img(&img),
                );
                Ok(())
            })?;
        }

        let score = &self.player.score;
        self.font.execute(|font| {
            let img = font.render(&format!("{:09}", score), style)?;
            window.draw(
                &Rectangle::new(
                    (WIN_WIDTH as f32 - img.area().width() - HUD_CORNER_PADDING, HUD_CORNER_PADDING),
                    img.area().size(),
                ),
                Background::Img(&img),
            );
            Ok(())
        })?;

        Ok(())
    }

    fn draw_player(&mut self, window: &mut Window) -> Result<()> {
        window.draw(
            &self.player.collector_rectangle().on_playfield(),
            Background::Col(Color::BLUE),
        );
        window.draw(
            &self.player.rect.on_playfield(),
            Background::Col(self.player.color),
        );

        Ok(())
    }
}

// Update logic
impl GameState {
    fn update_handle_input(&mut self, keyboard: &Keyboard) -> quicksilver::Result<()> {
        let movespeed = if keyboard[Key::LShift].is_down() {
            PLAYER_SPEED / PLAYER_SLOWMO_FACTOR
        } else {
            PLAYER_SPEED
        };

        // Check movement.
        if self.reset_countdown.is_none() {
            if keyboard[Key::H].is_down() || keyboard[Key::Left].is_down() {
                self.player.rect.pos.x -= movespeed;
            } else if keyboard[Key::J].is_down() || keyboard[Key::Down].is_down() {
                self.player.rect.pos.y += movespeed;
            } else if keyboard[Key::K].is_down() || keyboard[Key::Up].is_down() {
                self.player.rect.pos.y -= movespeed;
            } else if keyboard[Key::L].is_down() || keyboard[Key::Right].is_down() {
                self.player.rect.pos.x += movespeed;
            }
        }

        // Put player back in movement bounds.
        if self.player.rect.pos.x + self.player.rect.size.x > FIELD_EDGE_LENGTH {
            self.player.rect.pos.x = FIELD_EDGE_LENGTH - self.player.rect.size.x;
        } else if self.player.rect.pos.x < 0. {
            self.player.rect.pos.x = 0.;
        }
        if self.player.rect.pos.y + self.player.rect.size.y > FIELD_EDGE_LENGTH {
            self.player.rect.pos.y = FIELD_EDGE_LENGTH - self.player.rect.size.y;
        } else if self.player.rect.pos.y < 0. {
            self.player.rect.pos.y = 0.;
        }

        // Quit and shit.
        if keyboard[Key::Escape].is_down() {
            self.is_running = false;
        }

        Ok(())
    }

    fn update_fps_graph(&mut self, window: &Window) -> Result<()> {
        self.fps_graph.log_fps(window.current_fps());
        if self.fps_update_time.is_none()
            || Instant::now().duration_since(self.fps_update_time.unwrap())
            > Duration::from_millis(200)
        {
            self.fps_update_time = Some(Instant::now());
        }

        Ok(())
    }

    fn update_check_collisions(&mut self) -> Result<()> {
        if self.reset_countdown.is_none() {
            for ob in &mut self.obstacles {
                ob.lifetime += 1.;

                // Check collisions.
                if self.player.rect.overlaps_rectangle(&ob.rectangle()) {
                    self.reset_countdown = Some(Countdown::new(Duration::from_secs(2)));
                } else if self
                    .player
                    .collector_rectangle()
                    .overlaps_rectangle(&ob.rectangle())
                {
                    self.player.score += 1;
                }
            }
        }

        Ok(())
    }

    fn update_spawn_obstacles(&mut self) -> Result<()> {
        if self.last_spawned.is_none() || self.last_spawned.unwrap().elapsed() > self.spawn_interval
        {
            self.last_spawned = Some(Instant::now());
            self.obstacles.push(Obstacle::spawn(&mut self.rng));
            self.spawn_interval = GameState::obstacle_spawn_interval(self.player.score);
        }

        Ok(())
    }

    fn update_reset_game(&mut self) -> Result<()> {
        if let Some(c) = &self.reset_countdown {
            if c.is_done() {
                println!("You lose! Score: {}", self.player.score);
                self.obstacles.clear();
                self.player = Player::new();
                self.reset_countdown = None;
            }
        }

        Ok(())
    }

    fn update_despawn_obstacles(&mut self) -> Result<()> {
        // Give the player points and destroy an obstacle if it's offscreen.
        let player = &mut self.player;
        self.obstacles.retain(|&ob| {
            let res = ob.lifetime
                < ob.total_lifetime()
                + FIELD_EDGE_LENGTH / OBSTACLE_WARNING_MOVE_SPEED
                + OBSTACLE_HIDE_DELAY as f32;
            if !res {
                player.score += 100;
            }
            res
        });

        Ok(())
    }
}

impl State for GameState {
    fn new() -> quicksilver::Result<GameState> {
        Ok(GameState {
            obstacles: Vec::new(),
            player: Player::new(),
            rng: rand::thread_rng(),

            is_running: true,
            reset_countdown: None,

            fps_graph: FpsGraph::new(),
            fps_update_time: None,

            last_spawned: None,
            spawn_interval: Duration::new(4, 0),

            font: Asset::new(Font::load(FONT_NAME)),
            font_style: FontStyle::new(FONT_SIZE_PT, Color::WHITE),
        })
    }

    fn update(&mut self, window: &mut Window) -> quicksilver::Result<()> {
        fn update_inner(state: &mut GameState, window: &mut Window) -> Result<()> {
            if !state.is_running {
                window.close();
            }

            state.update_handle_input(window.keyboard())?;
            state.update_fps_graph(window)?;
            state.update_check_collisions()?;
            state.update_spawn_obstacles()?;
            state.update_despawn_obstacles()?;
            state.update_reset_game()?;

            Ok(())
        }

        update_inner(self, window).or_else(|e| {
            match e {
                Error::QuicksilverError(e) => return Err(e),
                _ => panic!(e.to_string())
            }
        })
    }

    fn draw(&mut self, window: &mut Window) -> quicksilver::Result<()> {
        fn draw_inner(state: &mut GameState, window: &mut Window) -> Result<()> {
            window.clear(Color::BLACK)?;

            state.draw_field_border(window)?;
            state.draw_player(window)?;
            state.draw_obstacles(window)?;
            state.draw_hud(window)?;

            Ok(())
        }

        draw_inner(self, window).or_else(|e| {
            match e {
                Error::QuicksilverError(e) => return Err(e),
                _ => panic!(e.to_string())
            }
        })
    }
}

/// Converts world-centric positioning to playfield-centric positioning.
trait ToPlayfieldCoordinates {
    fn on_playfield(&self) -> Rectangle;
}

impl ToPlayfieldCoordinates for Rectangle {
    fn on_playfield(&self) -> Rectangle {
        // This assumes the field is going in the center of the screen.
        self.translate((
            (WIN_WIDTH as f32 - FIELD_EDGE_LENGTH) / 2.,
            (WIN_HEIGHT as f32 - FIELD_EDGE_LENGTH) / 2.,
        ))
    }
}

fn main() {
    run::<GameState>(
        "First Game",
        Vector::new(WIN_WIDTH, WIN_HEIGHT),
        Settings::default(),
    );
}
