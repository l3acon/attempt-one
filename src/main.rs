// main.rs

use ggez::conf::{WindowMode, WindowSetup, NumSamples};
use ggez::event::{self, EventHandler, MouseButton};
use ggez::glam::Vec2;
use ggez::graphics::{self, Color, DrawMode, Mesh, MeshData, Rect, Text, TextLayout, Vertex};
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::{Context, ContextBuilder, GameResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Instant;

// Lyon imports
use lyon_path::path::Builder as LyonPathBuilder;
use lyon_path::math::Point as LyonPoint;
use lyon_tessellation::{
    StrokeTessellator, StrokeOptions, StrokeVertex, VertexBuffers,
    BuffersBuilder
};


// --- Configuration Structs ---
#[derive(Deserialize, Serialize, Debug)]
struct WindowConfig {
    width: f32,
    height: f32,
    title: String,
    msaa_level: Option<u8>, 
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ShapeConfig {
    width: f32,
    height: f32,
    corner_radius: f32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
    selection_outline_color_r: Option<u8>,
    selection_outline_color_g: Option<u8>,
    selection_outline_color_b: Option<u8>,
    selection_outline_width: Option<f32>,
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
const CONNECTOR_LINE_WIDTH: f32 = 2.0;
const CONNECTOR_LINE_COLOR: Color = Color::WHITE;
const CONNECTOR_CURVE_OFFSET: f32 = 40.0; 
const CONNECTOR_CIRCLE_RADIUS: f32 = 4.0; 
const CONNECTOR_CIRCLE_COLOR: Color = Color::WHITE;
const CONNECTOR_POINT_HORIZONTAL_OFFSET: f32 = 15.0; // Offset for line connection points from the left edge


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
    selection_outline_color: Color,
    selection_outline_width: f32,
    last_click_time: Option<Instant>,
    last_click_pos: Option<Vec2>,
    selected_shape_index: Option<usize>,
    dragged_shape_index: Option<usize>,
    drag_offset: Option<Vec2>,
    editing_shape_index: Option<usize>,
    current_input_text: String,
}

impl AppState {
    fn new(_ctx: &mut Context, shape_config: &ShapeConfig) -> GameResult<AppState> {
        let sel_color_r = shape_config.selection_outline_color_r.unwrap_or(255);
        let sel_color_g = shape_config.selection_outline_color_g.unwrap_or(255);
        let sel_color_b = shape_config.selection_outline_color_b.unwrap_or(0);
        let selection_outline_color = Color::from_rgb(sel_color_r, sel_color_g, sel_color_b);
        let selection_outline_width = shape_config.selection_outline_width.unwrap_or(2.0);

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
            selection_outline_color,
            selection_outline_width,
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

        // --- Draw Connector Lines and Circles ---
        if self.clicked_shapes.len() > 1 {
            for i in 0..(self.clicked_shapes.len() - 1) {
                let shape1_data = &self.clicked_shapes[i];
                let shape2_data = &self.clicked_shapes[i + 1];

                // Calculate bottom-left of shape1, with horizontal offset
                let start_x_f32 = (shape1_data.center_position.x - self.default_shape_width / 2.0) + CONNECTOR_POINT_HORIZONTAL_OFFSET;
                let start_y_f32 = shape1_data.center_position.y + self.default_shape_height / 2.0;
                let start_point_lyon = LyonPoint::new(start_x_f32, start_y_f32); 
                let start_point_ggez = Vec2::new(start_x_f32, start_y_f32); 

                // Calculate top-left of shape2, with horizontal offset
                let end_x_f32 = (shape2_data.center_position.x - self.default_shape_width / 2.0) + CONNECTOR_POINT_HORIZONTAL_OFFSET;
                let end_y_f32 = shape2_data.center_position.y - self.default_shape_height / 2.0;
                let end_point_lyon = LyonPoint::new(end_x_f32, end_y_f32); 
                let end_point_ggez = Vec2::new(end_x_f32, end_y_f32); 


                let direction_multiplier = if end_point_lyon.x > start_point_lyon.x { 1.0 } else { -1.0 };
                // Adjust control points to originate from the new offset start/end points
                let cp1 = LyonPoint::new(start_point_lyon.x + CONNECTOR_CURVE_OFFSET * direction_multiplier, start_point_lyon.y);
                let cp2 = LyonPoint::new(end_point_lyon.x - CONNECTOR_CURVE_OFFSET * direction_multiplier, end_point_lyon.y);


                let mut path_builder = LyonPathBuilder::new();
                path_builder.begin(start_point_lyon);
                path_builder.cubic_bezier_to(cp1, cp2, end_point_lyon);
                path_builder.end(false); 
                let lyon_path = path_builder.build();

                let mut geometry: VertexBuffers<Vertex, u32> = VertexBuffers::new();
                let mut stroke_tess = StrokeTessellator::new();
                let stroke_options = StrokeOptions::default().with_line_width(CONNECTOR_LINE_WIDTH);
                let line_color_arr = [
                    CONNECTOR_LINE_COLOR.r,
                    CONNECTOR_LINE_COLOR.g,
                    CONNECTOR_LINE_COLOR.b,
                    CONNECTOR_LINE_COLOR.a,
                ];

                stroke_tess.tessellate_path(
                    &lyon_path,
                    &stroke_options,
                    &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                        Vertex {
                            position: [vertex.position().x, vertex.position().y],
                            uv: [0.0, 0.0], 
                            color: line_color_arr,
                        }
                    }),
                ).unwrap_or_else(|e| {println!("Lyon tessellation error: {:?}", e);});


                if !geometry.vertices.is_empty() && !geometry.indices.is_empty() {
                    let mesh_data = MeshData {
                        vertices: &geometry.vertices, 
                        indices: &geometry.indices,   
                    };
                    let line_mesh = Mesh::from_data(ctx, mesh_data); 
                    canvas.draw(&line_mesh, graphics::DrawParam::default());

                    // Draw small circles at the (now offset) start and end of the connector line
                    let start_circle_mesh = Mesh::new_circle(
                        ctx,
                        DrawMode::fill(),
                        start_point_ggez, 
                        CONNECTOR_CIRCLE_RADIUS,
                        0.1, 
                        CONNECTOR_CIRCLE_COLOR,
                    )?;
                    canvas.draw(&start_circle_mesh, graphics::DrawParam::default());

                    let end_circle_mesh = Mesh::new_circle(
                        ctx,
                        DrawMode::fill(),
                        end_point_ggez, 
                        CONNECTOR_CIRCLE_RADIUS,
                        0.1, 
                        CONNECTOR_CIRCLE_COLOR,
                    )?;
                    canvas.draw(&end_circle_mesh, graphics::DrawParam::default());
                }
            }
        }

        // --- Draw Shapes, Outlines, and Text ---
        for (index, shape_data) in self.clicked_shapes.iter().enumerate() {
            let rect = Rect::new(
                shape_data.center_position.x - self.default_shape_width / 2.0,
                shape_data.center_position.y - self.default_shape_height / 2.0,
                self.default_shape_width,
                self.default_shape_height,
            );
            let shape_color_to_draw = self.default_shape_color;
            let rounded_rect_mesh = Mesh::new_rounded_rectangle(
                ctx,
                DrawMode::fill(),
                rect,
                self.default_shape_corner_radius,
                shape_color_to_draw,
            )?;
            canvas.draw(&rounded_rect_mesh, graphics::DrawParam::default());

            if self.selected_shape_index == Some(index) && self.editing_shape_index != Some(index) {
                let center_x = rect.x + rect.w / 2.0;
                let center_y = rect.y + rect.h / 2.0;
                let outline_w = rect.w * 1.05;
                let outline_h = rect.h * 1.05;
                let outline_bounds = Rect::new(
                    center_x - outline_w / 2.0,
                    center_y - outline_h / 2.0,
                    outline_w,
                    outline_h,
                );
                let outline_rect_mesh = Mesh::new_rounded_rectangle(
                    ctx,
                    DrawMode::stroke(self.selection_outline_width),
                    outline_bounds,
                    self.default_shape_corner_radius * 1.05,
                    self.selection_outline_color,
                )?;
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

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) -> GameResult {
        if button == MouseButton::Left {
            let current_click_time = Instant::now();
            let current_click_pos = Vec2::new(x, y);
            let mut clicked_on_shape_details: Option<(usize, Vec2)> = None;

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
                if self.editing_shape_index.is_some() && self.editing_shape_index != Some(clicked_idx) {
                    if let Some(editing_idx_val) = self.editing_shape_index.take() {
                        self.clicked_shapes[editing_idx_val].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                        self.current_input_text.clear();
                    }
                }
                self.selected_shape_index = Some(clicked_idx);
                let mut is_double_click_for_edit = false;
                if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                    if current_click_time.duration_since(last_time).as_millis() <= DOUBLE_CLICK_MAX_DELAY_MS && current_click_pos.distance(last_pos) <= DOUBLE_CLICK_MAX_DISTANCE {
                        is_double_click_for_edit = true;
                    }
                }
                if is_double_click_for_edit {
                    self.editing_shape_index = Some(clicked_idx);
                    self.current_input_text = self.clicked_shapes[clicked_idx].text.clone().unwrap_or_default();
                    self.dragged_shape_index = None;
                    self.last_click_time = None;
                    self.last_click_pos = None;
                } else {
                    self.dragged_shape_index = Some(clicked_idx);
                    self.drag_offset = Some(clicked_shape_center - current_click_pos);
                    self.last_click_time = Some(current_click_time);
                    self.last_click_pos = Some(current_click_pos);
                }
            } else {
                if let Some(editing_idx_val) = self.editing_shape_index.take() {
                    self.clicked_shapes[editing_idx_val].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                    self.current_input_text.clear();
                }
                self.selected_shape_index = None;
                self.dragged_shape_index = None;
                let mut is_double_click_for_create = false;
                if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_pos) {
                    if current_click_time.duration_since(last_time).as_millis() <= DOUBLE_CLICK_MAX_DELAY_MS && current_click_pos.distance(last_pos) <= DOUBLE_CLICK_MAX_DISTANCE {
                        is_double_click_for_create = true;
                    }
                }
                if is_double_click_for_create {
                    self.clicked_shapes.push(ShapeData { center_position: current_click_pos, text: None });
                    let new_idx = self.clicked_shapes.len() - 1;
                    self.selected_shape_index = Some(new_idx);
                    self.editing_shape_index = Some(new_idx);
                    self.current_input_text.clear();
                    self.last_click_time = None;
                    self.last_click_pos = None;
                } else {
                    self.last_click_time = Some(current_click_time);
                    self.last_click_pos = Some(current_click_pos);
                }
            }
        }
        Ok(())
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) -> GameResult {
        if button == MouseButton::Left && self.dragged_shape_index.is_some() {
            self.dragged_shape_index = None;
            self.drag_offset = None;
        }
        Ok(())
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) -> GameResult {
        self.live_mouse_pos = Vec2::new(x, y);
        if let Some(index) = self.dragged_shape_index {
            if let Some(offset) = self.drag_offset {
                if index < self.clicked_shapes.len() {
                    self.clicked_shapes[index].center_position = self.live_mouse_pos + offset;
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
                        if repeated { return Ok(()); }
                        if let Some(index) = self.editing_shape_index.take() {
                            self.clicked_shapes[index].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                            self.current_input_text.clear();
                            self.selected_shape_index = Some(index);
                        }
                    }
                    KeyCode::Escape => {
                        if repeated { return Ok(()); }
                        self.editing_shape_index = None;
                        self.current_input_text.clear();
                    }
                    KeyCode::Back => { self.current_input_text.pop(); }
                    KeyCode::Delete => {} 
                    _ => { if repeated { return Ok(()); } }
                }
            } else if let Some(index_to_delete) = self.selected_shape_index {
                if keycode == KeyCode::Delete && !repeated {
                    self.clicked_shapes.remove(index_to_delete);
                    self.selected_shape_index = None;
                    self.dragged_shape_index = None;
                    self.editing_shape_index = None;
                    self.last_click_time = None;
                    self.last_click_pos = None;
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
            title: "Rust: Shapes - Offset Connectors (Default)".to_string(), // Updated title
            msaa_level: None, 
        },
        shape: ShapeConfig {
            width: 120.0,
            height: 70.0,
            corner_radius: 10.0,
            color_r: 100,
            color_g: 200,
            color_b: 255,
            selection_outline_color_r: None,
            selection_outline_color_g: None,
            selection_outline_color_b: None,
            selection_outline_width: None,
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

    let msaa = match config.window.msaa_level {
        Some(1) => NumSamples::One, 
        Some(4) => NumSamples::Four,
        Some(other) => {
            println!(
                "Warning: Invalid msaa_level '{}' in config.toml. Valid options are 1 or 4. Defaulting to 4.",
                other
            );
            NumSamples::Four
        }
        None => NumSamples::Four, 
    };
    println!("Using MSAA level: {:?}", msaa);


    let (mut ctx, event_loop) = ContextBuilder::new("shapes_app_offset_connectors", "YourName") // Updated app name
        .window_setup(
            WindowSetup::default()
                .title(&config.window.title)
                .samples(msaa) 
        )
        .window_mode(
            WindowMode::default()
                .dimensions(config.window.width, config.window.height)
                .resizable(true)
        )
        .build()?;
    let app_state = AppState::new(&mut ctx, &config.shape)?;
    event::run(ctx, event_loop, app_state)
}

