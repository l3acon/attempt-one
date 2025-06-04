// main.rs

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, MouseButton};
use ggez::glam::Vec2; // For 2D vectors (coordinates)
use ggez::graphics::{self, Color, DrawMode, Mesh, Rect}; // Added Rect
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

// Renamed from CircleConfig to ShapeConfig and updated fields
#[derive(Deserialize, Serialize, Debug, Clone)]
struct ShapeConfig {
    width: f32,
    height: f32,
    corner_radius: f32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
}

#[derive(Deserialize, Serialize, Debug)]
struct AppConfig {
    window: WindowConfig,
    shape: ShapeConfig, // Renamed from circle to shape
}

// --- Constants for Double Click ---
const DOUBLE_CLICK_MAX_DELAY_MS: u128 = 500; // Max milliseconds between clicks
const DOUBLE_CLICK_MAX_DISTANCE: f32 = 10.0; // Max pixels mouse can move

// --- AppState Struct ---
struct AppState {
    live_mouse_pos: Vec2,
    clicked_shapes_positions: Vec<Vec2>, // Stores center positions of shapes
    shape_color: Color,
    shape_width: f32,
    shape_height: f32,
    shape_corner_radius: f32,
    last_click_time: Option<Instant>,
    last_click_pos: Option<Vec2>,
    dragged_shape_index: Option<usize>, // Index of the shape currently being dragged
    drag_offset: Option<Vec2>,
}

impl AppState {
    // Updated to take ShapeConfig
    fn new(_ctx: &mut Context, shape_config: &ShapeConfig) -> GameResult<AppState> {
        Ok(AppState {
            live_mouse_pos: Vec2::new(0.0, 0.0),
            clicked_shapes_positions: Vec::new(),
            shape_color: Color::from_rgb(
                shape_config.color_r,
                shape_config.color_g,
                shape_config.color_b,
            ),
            shape_width: shape_config.width,
            shape_height: shape_config.height,
            shape_corner_radius: shape_config.corner_radius,
            last_click_time: None,
            last_click_pos: None,
            dragged_shape_index: None,
            drag_offset: None,
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

        // Draw all stored rounded rectangles
        for &center_pos in &self.clicked_shapes_positions {
            // Calculate top-left corner for the Rect from the center position
            let rect = Rect::new(
                center_pos.x - self.shape_width / 2.0,
                center_pos.y - self.shape_height / 2.0,
                self.shape_width,
                self.shape_height,
            );

            let rounded_rect_mesh = Mesh::new_rounded_rectangle(
                ctx,
                DrawMode::fill(),
                rect, // Use the calculated Rect
                self.shape_corner_radius,
                self.shape_color,
            )?;
            canvas.draw(&rounded_rect_mesh, graphics::DrawParam::default());
        }

        let coords_text_string = format!(
            "Mouse: {:.0}, {:.0} | Shapes: {}", // Changed "Circles" to "Shapes"
            self.live_mouse_pos.x,
            self.live_mouse_pos.y,
            self.clicked_shapes_positions.len()
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
            let mut clicked_on_shape_to_drag = false;

            // 1. Check if we clicked on an existing shape to start a drag.
            for (index, &shape_center) in self.clicked_shapes_positions.iter().enumerate().rev() {
                // Define the rectangle for hit detection
                let shape_rect = Rect::new(
                    shape_center.x - self.shape_width / 2.0,
                    shape_center.y - self.shape_height / 2.0,
                    self.shape_width,
                    self.shape_height,
                );

                if shape_rect.contains(current_click_pos) { // Use Rect::contains for hit detection
                    self.dragged_shape_index = Some(index);
                    self.drag_offset = Some(shape_center - current_click_pos);
                    clicked_on_shape_to_drag = true;
                    println!("Starting drag for shape at index {}", index);

                    self.last_click_time = None;
                    self.last_click_pos = None;
                    break;
                }
            }

            // 2. If not dragging, handle potential double-click for creation.
            if !clicked_on_shape_to_drag {
                if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                    let duration_since_last = current_click_time.duration_since(last_time).as_millis();
                    let distance_from_last = current_click_pos.distance(last_pos);

                    if duration_since_last <= DOUBLE_CLICK_MAX_DELAY_MS
                        && distance_from_last <= DOUBLE_CLICK_MAX_DISTANCE
                    {
                        println!(
                            "Double click on empty space at: ({}, {}) - New shape added.",
                            current_click_pos.x, current_click_pos.y
                        );
                        self.clicked_shapes_positions.push(current_click_pos); // Store center position

                        self.last_click_time = None;
                        self.last_click_pos = None;
                    } else {
                        self.last_click_time = Some(current_click_time);
                        self.last_click_pos = Some(current_click_pos);
                        println!("Single click on empty space at: ({}, {}) (potential first of double)", x, y);
                    }
                } else {
                    self.last_click_time = Some(current_click_time);
                    self.last_click_pos = Some(current_click_pos);
                    println!("Single click on empty space at: ({}, {}) (potential first of double)", x, y);
                }
            }
        }
        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        if button == MouseButton::Left {
            if self.dragged_shape_index.is_some() {
                println!("Dropped shape at: ({:.0}, {:.0})", self.live_mouse_pos.x, self.live_mouse_pos.y);
                self.dragged_shape_index = None;
                self.drag_offset = None;
            }
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

        if let Some(index) = self.dragged_shape_index {
            if let Some(offset) = self.drag_offset {
                let new_center = self.live_mouse_pos + offset;
                if index < self.clicked_shapes_positions.len() {
                    self.clicked_shapes_positions[index] = new_center;
                }
            }
        }
        Ok(())
    }
}

// --- Configuration Loading Function ---
fn load_config() -> AppConfig {
    // Updated default configuration for shapes
    let default_config = AppConfig {
        window: WindowConfig {
            width: 800.0,
            height: 600.0,
            title: "Rust: Drag & Double Click Rounded Rects (Default)".to_string(),
        },
        shape: ShapeConfig { // Renamed to shape
            width: 100.0,
            height: 60.0,
            corner_radius: 10.0,
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

    let (mut ctx, event_loop) = ContextBuilder::new("configurable_rounded_rects_app", "YourName")
        .window_setup(WindowSetup::default().title(&config.window.title))
        .window_mode(
            WindowMode::default()
                .dimensions(config.window.width, config.window.height)
                .resizable(true),
        )
        .build()?;

    // Pass the shape config to AppState::new
    let app_state = AppState::new(&mut ctx, &config.shape)?;

    event::run(ctx, event_loop, app_state)
}
