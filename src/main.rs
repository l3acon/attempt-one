// main.rs

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, MouseButton};
use ggez::glam::Vec2; // For 2D vectors (coordinates)
use ggez::graphics::{self, Color, DrawMode, Mesh};
use ggez::{Context, ContextBuilder, GameResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Instant; // For tracking click times

// --- Configuration Structs ---
#[derive(Deserialize, Serialize, Debug)]
struct WindowConfig {
    width: f32,
    height: f32,
    title: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct CircleConfig {
    radius: f32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
}

#[derive(Deserialize, Serialize, Debug)]
struct AppConfig {
    window: WindowConfig,
    circle: CircleConfig,
}

// --- Constants for Double Click ---
const DOUBLE_CLICK_MAX_DELAY_MS: u128 = 500; // Max milliseconds between clicks
const DOUBLE_CLICK_MAX_DISTANCE: f32 = 10.0; // Max pixels mouse can move

// --- AppState Struct ---
struct AppState {
    live_mouse_pos: Vec2,
    clicked_circles_positions: Vec<Vec2>,
    circle_color: Color,
    circle_radius: f32,
    last_click_time: Option<Instant>, // Time of the last click
    last_click_pos: Option<Vec2>,   // Position of the last click
}

impl AppState {
    fn new(_ctx: &mut Context, circle_config: &CircleConfig) -> GameResult<AppState> {
        Ok(AppState {
            live_mouse_pos: Vec2::new(0.0, 0.0),
            clicked_circles_positions: Vec::new(),
            circle_color: Color::from_rgb(
                circle_config.color_r,
                circle_config.color_g,
                circle_config.color_b,
            ),
            circle_radius: circle_config.radius,
            last_click_time: None,
            last_click_pos: None,
        })
    }
}

// --- EventHandler Implementation ---
impl EventHandler<ggez::GameError> for AppState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::from_rgb(30, 30, 40));

        for &pos in &self.clicked_circles_positions {
            let circle_mesh = Mesh::new_circle(
                ctx,
                DrawMode::fill(),
                pos,
                self.circle_radius,
                0.1,
                self.circle_color,
            )?;
            canvas.draw(&circle_mesh, graphics::DrawParam::default());
        }

        let coords_text_string = format!(
            "Mouse: {:.0}, {:.0} | Circles: {}",
            self.live_mouse_pos.x,
            self.live_mouse_pos.y,
            self.clicked_circles_positions.len()
        );
        let mut text_display = graphics::Text::new(coords_text_string);
        text_display.set_scale(20.0);

        canvas.draw(
            &text_display,
            graphics::DrawParam::default()
                .dest(Vec2::new(10.0, 10.0))
                .color(Color::WHITE),
        );

        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        if button == MouseButton::Left {
            let current_click_time = Instant::now();
            let current_click_pos = Vec2::new(x, y);

            if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                let duration_since_last = current_click_time.duration_since(last_time).as_millis();
                let distance_from_last = current_click_pos.distance(last_pos);

                if duration_since_last <= DOUBLE_CLICK_MAX_DELAY_MS
                    && distance_from_last <= DOUBLE_CLICK_MAX_DISTANCE
                {
                    // This is a double click!
                    println!(
                        "Double click at: ({}, {}) - New circle added.",
                        current_click_pos.x, current_click_pos.y
                    );
                    self.clicked_circles_positions.push(current_click_pos);

                    // Reset last click info to prevent the second click of this double-click
                    // from becoming the first click of a new potential double-click.
                    self.last_click_time = None;
                    self.last_click_pos = None;
                } else {
                    // Not a double click (too slow or too far), treat as a new first click.
                    self.last_click_time = Some(current_click_time);
                    self.last_click_pos = Some(current_click_pos);
                    println!("Single click at: ({}, {}) (potential first of double)", x, y);
                }
            } else {
                // This is the first click (or first after a successful double click).
                self.last_click_time = Some(current_click_time);
                self.last_click_pos = Some(current_click_pos);
                println!("Single click at: ({}, {}) (potential first of double)", x, y);
            }
        }
        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        if button == MouseButton::Left {
            // This event is less critical for the double-click logic itself,
            // but can be used for logging or other actions.
            // println!("Left mouse button released at: ({}, {})", x, y);
        }
        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        self.live_mouse_pos = Vec2::new(x, y);
        Ok(())
    }
}

// --- Configuration Loading Function ---
fn load_config() -> AppConfig {
    let default_config = AppConfig {
        window: WindowConfig {
            width: 800.0,
            height: 600.0,
            title: "Rust: Double Click to Add Circles (Default Config)".to_string(),
        },
        circle: CircleConfig {
            radius: 30.0,
            color_r: 100,
            color_g: 200,
            color_b: 255,
        },
    };

    let config_path = "config.toml";

    match fs::read_to_string(config_path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => {
                println!("Successfully loaded configuration from {}", config_path);
                config
            }
            Err(e) => {
                eprintln!(
                    "Failed to parse {}: {}. Using default configuration.",
                    config_path, e
                );
                default_config
            }
        },
        Err(_) => {
            println!(
                "{} not found. Using default configuration and creating a new one.",
                config_path
            );
            match toml::to_string_pretty(&default_config) {
                Ok(toml_string) => {
                    if let Err(e) = fs::write(config_path, toml_string) {
                        eprintln!("Could not write default {}: {}", config_path, e);
                    } else {
                        println!("Default {} created.", config_path);
                    }
                }
                Err(e) => {
                    eprintln!("Could not serialize default config: {}", e);
                }
            }
            default_config
        }
    }
}

// --- Main Function ---
pub fn main() -> GameResult {
    let config = load_config();

    let (mut ctx, event_loop) = ContextBuilder::new("configurable_circles_app_double_click", "YourName")
        .window_setup(WindowSetup::default().title(&config.window.title))
        .window_mode(
            WindowMode::default()
                .dimensions(config.window.width, config.window.height)
                .resizable(true),
        )
        .build()?;

    let app_state = AppState::new(&mut ctx, &config.circle)?;

    event::run(ctx, event_loop, app_state)
}
