// main.rs

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, MouseButton};
use ggez::glam::Vec2; // For 2D vectors (coordinates)
use ggez::graphics::{self, Color, DrawMode, Mesh, Rect, Text, TextLayout};
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::{Context, ContextBuilder, GameResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Instant;

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

// --- Constants ---
const DOUBLE_CLICK_MAX_DELAY_MS: u128 = 500;
const DOUBLE_CLICK_MAX_DISTANCE: f32 = 10.0;
const TEXT_PADDING: f32 = 8.0;

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

    default_shape_color: Color,
    default_shape_width: f32,
    default_shape_height: f32,
    default_shape_corner_radius: f32,

    last_click_time: Option<Instant>,
    last_click_pos: Option<Vec2>,

    selected_shape_index: Option<usize>, // Index of the currently selected shape
    dragged_shape_index: Option<usize>,
    drag_offset: Option<Vec2>,

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
            selected_shape_index: None, 
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
            // This is the main rectangle for the shape's fill
            let rect = Rect::new(
                shape_data.center_position.x - self.default_shape_width / 2.0,
                shape_data.center_position.y - self.default_shape_height / 2.0,
                self.default_shape_width,
                self.default_shape_height,
            );

            let shape_color_to_draw = self.default_shape_color;
            // Note: Visual feedback for selection is now handled by drawing a separate outline below.

            let rounded_rect_mesh = Mesh::new_rounded_rectangle(
                ctx,
                DrawMode::fill(),
                rect, // Use the original rect for the main shape
                self.default_shape_corner_radius,
                shape_color_to_draw,
            )?;
            canvas.draw(&rounded_rect_mesh, graphics::DrawParam::default());

            // Draw selection outline if selected and not editing
            if self.selected_shape_index == Some(index) && self.editing_shape_index != Some(index) {
                // Calculate properties for the outline rect, centered on the original rect
                let center_x = rect.x + rect.w / 2.0;
                let center_y = rect.y + rect.h / 2.0;
                let outline_w = rect.w * 1.05; // Make outline slightly wider
                let outline_h = rect.h * 1.05; // Make outline slightly taller

                // Create a new Rect for the outline's bounds
                let outline_bounds = Rect::new(
                    center_x - outline_w / 2.0,
                    center_y - outline_h / 2.0,
                    outline_w,
                    outline_h
                );

                let outline_rect_mesh = Mesh::new_rounded_rectangle(
                    ctx,
                    DrawMode::stroke(2.0), // Stroke width for the outline
                    outline_bounds,        // Use the correctly defined Rect for the outline
                    self.default_shape_corner_radius * 1.05, // Optionally scale corner radius
                    Color::YELLOW,         // Outline color
                )?;
                // Draw the mesh. Since its bounds are already in world coordinates,
                // DrawParam::default() is sufficient.
                canvas.draw(&outline_rect_mesh, graphics::DrawParam::default());
            }


            let text_to_display = if self.editing_shape_index == Some(index) {
                format!("{}|", self.current_input_text)
            } else {
                shape_data.text.clone().unwrap_or_default()
            };

            if !text_to_display.is_empty() {
                let wrap_width = self.default_shape_width - (TEXT_PADDING * 2.0);
                let mut text_obj = Text::new(text_to_display);
                text_obj.set_layout(TextLayout::center());
                text_obj.set_scale(18.0);
                text_obj.set_bounds(Vec2::new(wrap_width, f32::INFINITY));
                let text_dest = shape_data.center_position;
                canvas.draw(&text_obj, graphics::DrawParam::default().dest(text_dest).color(Color::BLACK));
            }
        }

        let status_text = format!(
            "Mouse: {:.0}, {:.0} | Shapes: {} {}{}",
            self.live_mouse_pos.x,
            self.live_mouse_pos.y,
            self.clicked_shapes.len(),
            if self.editing_shape_index.is_some() { "[EDITING]" } else { "" },
            if self.selected_shape_index.is_some() && self.editing_shape_index.is_none() { "[SELECTED]" } else { "" }
        );
        let mut text_display = graphics::Text::new(status_text);
        text_display.set_scale(20.0);
        canvas.draw(&text_display, graphics::DrawParam::default().dest(Vec2::new(10.0, 10.0)).color(Color::WHITE));

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
            
            let mut clicked_on_shape_details: Option<(usize, Vec2)> = None; // (index, center_position_of_clicked_shape)

            // Phase 1: Identify if a click landed on any shape
            for (index, shape_data) in self.clicked_shapes.iter().enumerate().rev() {
                let shape_rect = Rect::new(
                    shape_data.center_position.x - self.default_shape_width / 2.0,
                    shape_data.center_position.y - self.default_shape_height / 2.0,
                    self.default_shape_width,
                    self.default_shape_height,
                );
                if shape_rect.contains(current_click_pos) {
                    clicked_on_shape_details = Some((index, shape_data.center_position));
                    break;
                }
            }

            if let Some((clicked_idx, clicked_shape_center)) = clicked_on_shape_details {
                // --- Click was on an existing shape ---
                // 1. Finalize editing if a *different* shape was being edited
                if self.editing_shape_index.is_some() && self.editing_shape_index != Some(clicked_idx) {
                    if let Some(editing_idx_val) = self.editing_shape_index.take() {
                        self.clicked_shapes[editing_idx_val].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                        self.current_input_text.clear();
                        println!("Finalized text for shape {} due to click on another shape.", editing_idx_val);
                    }
                }
                
                self.selected_shape_index = Some(clicked_idx); // Select the clicked shape

                // 2. Check for double-click on this shape to start/continue editing
                let mut is_double_click_for_edit = false;
                if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                    let duration = current_click_time.duration_since(last_time).as_millis();
                    let distance = current_click_pos.distance(last_pos);
                    if duration <= DOUBLE_CLICK_MAX_DELAY_MS && distance <= DOUBLE_CLICK_MAX_DISTANCE {
                        is_double_click_for_edit = true;
                    }
                }

                if is_double_click_for_edit {
                    println!("Double-click on shape {}: starting text edit.", clicked_idx);
                    self.editing_shape_index = Some(clicked_idx);
                    self.current_input_text = self.clicked_shapes[clicked_idx].text.clone().unwrap_or_default();
                    self.dragged_shape_index = None; 
                    self.last_click_time = None; 
                    self.last_click_pos = None;
                } else {
                    // Single click on this shape: start dragging
                    println!("Single-click on shape {}: selected. Starting drag.", clicked_idx);
                    self.dragged_shape_index = Some(clicked_idx);
                    self.drag_offset = Some(clicked_shape_center - current_click_pos);
                    self.last_click_time = Some(current_click_time);
                    self.last_click_pos = Some(current_click_pos);
                }

            } else {
                // --- Click was on empty space ---
                // 1. Finalize editing if any shape was being edited
                if let Some(editing_idx_val) = self.editing_shape_index.take() {
                    self.clicked_shapes[editing_idx_val].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                    self.current_input_text.clear();
                    println!("Clicked empty space: finalized text for shape {}.", editing_idx_val);
                }
                
                self.selected_shape_index = None; // Deselect any shape
                self.dragged_shape_index = None; // Stop any drag if clicked on empty space

                // 2. Check for double-click on empty space to create new shape
                let mut is_double_click_for_create = false;
                if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                    let duration = current_click_time.duration_since(last_time).as_millis();
                    let distance = current_click_pos.distance(last_pos);
                    if duration <= DOUBLE_CLICK_MAX_DELAY_MS && distance <= DOUBLE_CLICK_MAX_DISTANCE {
                        is_double_click_for_create = true;
                    }
                }

                if is_double_click_for_create {
                    println!("Double-click on empty space: New shape added & now editing.");
                    self.clicked_shapes.push(ShapeData { center_position: current_click_pos, text: None });
                    let new_idx = self.clicked_shapes.len() - 1;
                    self.selected_shape_index = Some(new_idx); // New shape is selected
                    self.editing_shape_index = Some(new_idx); // Start editing it
                    self.current_input_text.clear();
                    self.last_click_time = None; 
                    self.last_click_pos = None;
                } else {
                    // Single click on empty space: record for potential next double-click
                    println!("Single click on empty space.");
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
                // If a drag was happening, it ends. The shape remains selected.
                println!("Dropped shape {}", self.dragged_shape_index.unwrap());
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
            // Handle text editing keys first if editing_shape_index is Some
            if self.editing_shape_index.is_some() {
                match keycode {
                    KeyCode::Return | KeyCode::NumpadEnter => {
                        if repeated { return Ok(()); }
                        if let Some(index) = self.editing_shape_index.take() {
                            self.clicked_shapes[index].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                            self.current_input_text.clear();
                            println!("Text editing finished for shape {}.", index);
                            // After finishing edit, the shape remains selected but not editing.
                            self.selected_shape_index = Some(index); 
                        }
                    }
                    KeyCode::Escape => {
                        if repeated { return Ok(()); }
                        self.editing_shape_index = None;
                        self.current_input_text.clear();
                        println!("Text editing cancelled.");
                        // Shape might still be selected if it was before editing started.
                    }
                    KeyCode::Back => { // Allows repeat for continuous delete
                        self.current_input_text.pop();
                    }
                    KeyCode::Delete => {
                        println!("Delete pressed during text edit - no action on shape.");
                    }
                    _ => { if repeated { return Ok(()); } }
                }
            } else {
                // Not editing text, so keys can affect selected shapes
                if let Some(index_to_delete) = self.selected_shape_index {
                    if keycode == KeyCode::Delete {
                        if repeated { return Ok(()); } 
                        
                        println!("Delete key pressed for selected shape index {}", index_to_delete);
                        self.clicked_shapes.remove(index_to_delete);
                        
                        self.selected_shape_index = None;
                        self.dragged_shape_index = None; 
                        self.editing_shape_index = None; 
                        self.last_click_time = None; 
                        self.last_click_pos = None;
                        println!("Shape deleted.");
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
            title: "Rust: Shapes - Text, Drag, Delete (Default)".to_string(),
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
    let (mut ctx, event_loop) = ContextBuilder::new("shapes_app_delete", "YourName")
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

