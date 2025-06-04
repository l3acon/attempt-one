// main.rs

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, MouseButton};
use ggez::glam::Vec2; // For 2D vectors (coordinates)
use ggez::graphics::{self, Color, DrawMode, Mesh, Rect, Text, TextLayout}; // Added Text, TextLayout
use ggez::input::keyboard::{KeyCode, KeyInput}; // Added KeyCode, KeyInput
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
    shape: ShapeConfig,
}

// --- Constants for Double Click ---
const DOUBLE_CLICK_MAX_DELAY_MS: u128 = 500;
const DOUBLE_CLICK_MAX_DISTANCE: f32 = 10.0;
const TEXT_PADDING: f32 = 8.0; // Padding from the rectangle's edge for the text

// --- Data structure for individual shapes ---
#[derive(Clone, Debug)]
struct ShapeData {
    center_position: Vec2,
    text: Option<String>,
}

// --- AppState Struct ---
struct AppState {
    live_mouse_pos: Vec2,
    clicked_shapes: Vec<ShapeData>,

    // Default properties for new shapes from config
    default_shape_color: Color,
    default_shape_width: f32,
    default_shape_height: f32,
    default_shape_corner_radius: f32,

    // Double-click detection
    last_click_time: Option<Instant>,
    last_click_pos: Option<Vec2>,

    // Dragging state
    dragged_shape_index: Option<usize>,
    drag_offset: Option<Vec2>,

    // Text editing state
    editing_shape_index: Option<usize>,
    current_input_text: String,
}

impl AppState {
    fn new(_ctx: &mut Context, shape_config: &ShapeConfig) -> GameResult<AppState> {
        Ok(AppState {
            live_mouse_pos: Vec2::new(0.0, 0.0),
            clicked_shapes: Vec::new(),
            default_shape_color: Color::from_rgb(
                shape_config.color_r,
                shape_config.color_g,
                shape_config.color_b,
            ),
            default_shape_width: shape_config.width,
            default_shape_height: shape_config.height,
            default_shape_corner_radius: shape_config.corner_radius,
            last_click_time: None,
            last_click_pos: None,
            dragged_shape_index: None,
            drag_offset: None,
            editing_shape_index: None,
            current_input_text: String::new(),
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

        for (index, shape_data) in self.clicked_shapes.iter().enumerate() {
            let rect = Rect::new(
                shape_data.center_position.x - self.default_shape_width / 2.0,
                shape_data.center_position.y - self.default_shape_height / 2.0,
                self.default_shape_width,
                self.default_shape_height,
            );

            let rounded_rect_mesh = Mesh::new_rounded_rectangle(
                ctx,
                DrawMode::fill(),
                rect,
                self.default_shape_corner_radius,
                self.default_shape_color,
            )?;
            canvas.draw(&rounded_rect_mesh, graphics::DrawParam::default());

            // Determine text to display for this shape
            let text_to_display = if self.editing_shape_index == Some(index) {
                format!("{}|", self.current_input_text) // Simple cursor
            } else {
                shape_data.text.clone().unwrap_or_default()
            };

            // Only proceed with text drawing if there's text to show
            if !text_to_display.is_empty() {
                // Calculate the maximum width for the text block
                let wrap_width = self.default_shape_width - (TEXT_PADDING * 2.0);

                let mut text_obj = Text::new(text_to_display);
                text_obj.set_layout(TextLayout::center()); // Center the text block horizontally and vertically
                text_obj.set_scale(18.0);
                // Set the bounds to enable text wrapping
                // We provide the maximum width and infinite height so it wraps as many lines as needed.
                text_obj.set_bounds(Vec2::new(wrap_width, f32::INFINITY));

                // Position the text at the center of the shape
                let text_dest = shape_data.center_position;
                canvas.draw(&text_obj, graphics::DrawParam::default().dest(text_dest).color(Color::BLACK));
            }
        }

        let coords_text_string = format!(
            "Mouse: {:.0}, {:.0} | Shapes: {} {}",
            self.live_mouse_pos.x,
            self.live_mouse_pos.y,
            self.clicked_shapes.len(),
            if self.editing_shape_index.is_some() { "[EDITING]" } else { "" }
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
            let mut interacted_with_existing_shape = false;

            for (index, shape_data) in self.clicked_shapes.iter().enumerate().rev() {
                let shape_rect = Rect::new(
                    shape_data.center_position.x - self.default_shape_width / 2.0,
                    shape_data.center_position.y - self.default_shape_height / 2.0,
                    self.default_shape_width,
                    self.default_shape_height,
                );

                if shape_rect.contains(current_click_pos) {
                    interacted_with_existing_shape = true;
                    if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                        let duration_since_last = current_click_time.duration_since(last_time).as_millis();
                        let distance_from_last = current_click_pos.distance(last_pos);

                        if duration_since_last <= DOUBLE_CLICK_MAX_DELAY_MS && distance_from_last <= DOUBLE_CLICK_MAX_DISTANCE {
                            println!("Double-click on shape {}: starting text edit.", index);
                            self.editing_shape_index = Some(index);
                            self.current_input_text = shape_data.text.clone().unwrap_or_default();
                            self.dragged_shape_index = None;
                            self.last_click_time = None;
                            self.last_click_pos = None;
                            break;
                        }
                    }

                    if self.editing_shape_index != Some(index) {
                        println!("Single-click on shape {}: starting drag.", index);
                        self.dragged_shape_index = Some(index);
                        self.drag_offset = Some(shape_data.center_position - current_click_pos);
                        self.last_click_time = Some(current_click_time);
                        self.last_click_pos = Some(current_click_pos);
                    }
                    break;
                }
            }

            if !interacted_with_existing_shape {
                if self.editing_shape_index.is_some() {
                    if let Some(idx) = self.editing_shape_index.take() {
                        self.clicked_shapes[idx].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                        self.current_input_text.clear();
                        println!("Clicked empty space while editing: text saved for shape {}.", idx);
                    }
                }

                if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                    let duration_since_last = current_click_time.duration_since(last_time).as_millis();
                    let distance_from_last = current_click_pos.distance(last_pos);

                    if duration_since_last <= DOUBLE_CLICK_MAX_DELAY_MS && distance_from_last <= DOUBLE_CLICK_MAX_DISTANCE {
                        println!("Double-click on empty space: New shape added & now editing.");
                        self.clicked_shapes.push(ShapeData {
                            center_position: current_click_pos,
                            text: None,
                        });
                        let new_shape_index = self.clicked_shapes.len() - 1;
                        self.editing_shape_index = Some(new_shape_index);
                        self.current_input_text.clear();
                        self.dragged_shape_index = None;
                        self.last_click_time = None;
                        self.last_click_pos = None;
                    } else {
                        self.last_click_time = Some(current_click_time);
                        self.last_click_pos = Some(current_click_pos);
                    }
                } else {
                    self.last_click_time = Some(current_click_time);
                    self.last_click_pos = Some(current_click_pos);
                }
            }
        }
        Ok(())
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) -> GameResult {
        if button == MouseButton::Left {
            if self.dragged_shape_index.is_some() {
                println!("Dropped shape at: ({:.0}, {:.0})", self.live_mouse_pos.x, self.live_mouse_pos.y);
                self.dragged_shape_index = None;
                self.drag_offset = None;
            }
        }
        Ok(())
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) -> GameResult {
        self.live_mouse_pos = Vec2::new(x, y);
        if let Some(index) = self.dragged_shape_index {
            if let Some(offset) = self.drag_offset {
                let new_center = self.live_mouse_pos + offset;
                if index < self.clicked_shapes.len() {
                    self.clicked_shapes[index].center_position = new_center;
                }
            }
        }
        Ok(())
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) -> GameResult {
        if self.editing_shape_index.is_some() && !character.is_control() {
            self.current_input_text.push(character);
        }
        Ok(())
    }

    fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, repeated: bool) -> GameResult {
        if let Some(keycode) = input.keycode {
            if self.editing_shape_index.is_some() {
                match keycode {
                    KeyCode::Return | KeyCode::NumpadEnter => {
                        // Prevent Return/Enter from repeating if key is held down
                        if repeated {
                            return Ok(());
                        }
                        if let Some(index) = self.editing_shape_index.take() {
                            self.clicked_shapes[index].text = if self.current_input_text.is_empty() {
                                None
                            } else {
                                Some(self.current_input_text.clone())
                            };
                            self.current_input_text.clear();
                            println!("Text editing finished for shape {}.", index);
                        }
                    }
                    KeyCode::Escape => {
                        // Prevent Escape from repeating if key is held down
                        if repeated {
                            return Ok(());
                        }
                        self.editing_shape_index = None;
                        self.current_input_text.clear();
                        println!("Text editing cancelled.");
                    }
                    KeyCode::Back => {
                        // Allow Backspace to repeat (i.e., process it even if `repeated` is true)
                        // The OS handles the repeat rate for us.
                        self.current_input_text.pop();
                    }
                    _ => {
                        // For any other key press during text editing that isn't handled above,
                        // if it's a repeated event, we can ignore it here as actual character input
                        // is handled by `text_input_event`.
                        if repeated {
                            return Ok(());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn load_config() -> AppConfig {
    let default_config = AppConfig {
        window: WindowConfig {
            width: 800.0,
            height: 600.0,
            title: "Rust: Shapes with Text (Default Config)".to_string(),
        },
        shape: ShapeConfig {
            width: 120.0,
            height: 70.0,
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
                eprintln!("Failed to parse {}: {}. Using default.", config_path, e);
                default_config
            }
        },
        Err(_) => {
            println!("{} not found. Using default & creating new one.", config_path);
            match toml::to_string_pretty(&default_config) {
                Ok(toml_string) => {
                    if let Err(e) = fs::write(config_path, toml_string) {
                        eprintln!("Could not write default {}: {}", config_path, e);
                    } else {
                        println!("Default {} created.", config_path);
                    }
                }
                Err(e) => eprintln!("Could not serialize default config: {}", e),
            }
            default_config
        }
    }
}

pub fn main() -> GameResult {
    let config = load_config();
    let (mut ctx, event_loop) = ContextBuilder::new("shapes_with_text_app", "YourName")
        .window_setup(WindowSetup::default().title(&config.window.title))
        .window_mode(
            WindowMode::default()
                .dimensions(config.window.width, config.window.height)
                .resizable(true),
        )
        .build()?;
    let app_state = AppState::new(&mut ctx, &config.shape)?;
    event::run(ctx, event_loop, app_state)
}
