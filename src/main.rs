// main.rs

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, MouseButton};
use ggez::glam::Vec2; // For 2D vectors (coordinates)
use ggez::graphics::{self, Color, DrawMode, Mesh};
use ggez::{Context, ContextBuilder, GameResult};
use serde::{Deserialize, Serialize}; // Import Serialize
use std::fs; // For file system operations

// --- Configuration Structs ---
// Added Serialize to the derive macro for all config structs
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

// --- AppState Struct ---
struct AppState {
    live_mouse_pos: Vec2,
    clicked_circles_positions: Vec<Vec2>,
    circle_color: Color, 
    circle_radius: f32,  
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
        })
    }
}

// --- EventHandler Implementation (mostly unchanged) ---
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
            let click_pos = Vec2::new(x, y);
            println!(
                "Left mouse button pressed at: ({}, {}) - New circle added.",
                click_pos.x, click_pos.y
            );
            self.clicked_circles_positions.push(click_pos);
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
            println!("Left mouse button released at: ({}, {})", x, y);
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
            title: "Rust: Click to Add Circles (Default Config)".to_string(),
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
            // Attempt to create a default config.toml file
            match toml::to_string_pretty(&default_config) { // This requires Serialize
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

    let (mut ctx, event_loop) = ContextBuilder::new("configurable_circles_app", "YourName")
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
