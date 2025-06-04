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
#[derive(Deserialize, Serialize, Debug, Clone)] 
struct ColorsConfig {
    connector_line_rgb: Option<[u8; 3]>,
    selected_connector_line_rgb: Option<[u8; 3]>,
    preview_connector_line_rgb: Option<[u8; 3]>, // Alpha will be hardcoded
    default_port_rgb: Option<[u8; 3]>,
    selected_connector_port_rgb: Option<[u8; 3]>,
    active_new_line_start_port_rgb: Option<[u8; 3]>,
}

impl Default for ColorsConfig {
    fn default() -> Self {
        ColorsConfig {
            connector_line_rgb: None,
            selected_connector_line_rgb: None,
            preview_connector_line_rgb: None,
            default_port_rgb: None,
            selected_connector_port_rgb: None,
            active_new_line_start_port_rgb: None,
        }
    }
}


#[derive(Deserialize, Serialize, Debug)]
struct WindowConfig {
    width: f32,
    height: f32,
    title: String,
    msaa_level: Option<u8>, 
    ui_scale_factor: Option<f32>, 
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ShapeConfig {
    width: f32,
    height: f32,
    corner_radius: f32,
    base_color_rgb: [u8; 3], // Changed from color_r, color_g, color_b
    selection_outline_color_rgb: Option<[u8; 3]>, // Changed from _r, _g, _b options
    selection_outline_width: Option<f32>,
}

#[derive(Deserialize, Serialize, Debug)]
struct AppConfig {
    window: WindowConfig,
    shape: ShapeConfig,
    colors: Option<ColorsConfig>, 
}

// --- Constants for non-color visual properties ---
const DOUBLE_CLICK_MAX_DELAY_MS: u128 = 500;
const DOUBLE_CLICK_MAX_DISTANCE: f32 = 10.0;
const TEXT_PADDING: f32 = 8.0;
const CONNECTOR_LINE_WIDTH: f32 = 2.0;
const CONNECTOR_CURVE_OFFSET: f32 = 40.0; 

const PORT_DRAW_RADIUS_DEFAULT: f32 = 4.0; 
const PORT_DRAW_RADIUS_HOVER: f32 = 8.0;  
const PORT_CLICK_RADIUS: f32 = 8.0;     
const PORT_HOVER_DETECT_DISTANCE: f32 = 15.0; 

const CONNECTOR_POINT_HORIZONTAL_OFFSET: f32 = 15.0;
const CONNECTOR_SELECTION_RADIUS: f32 = CONNECTOR_LINE_WIDTH * 4.0; 
const CONNECTOR_SAMPLE_POINTS: usize = 10;


// --- Data structure for individual shapes ---
#[derive(Clone, Debug)]
struct ShapeData {
    center_position: Vec2,
    text: Option<String>,
}

// --- Data structure for user-defined connections ---
#[derive(Clone, Debug, PartialEq, Eq)]
struct UserConnection {
    from_shape_index: usize,
    to_shape_index: usize,
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
    
    ui_scale: f32, 

    // Colors loaded from config or defaulted
    connector_line_color: Color,
    selected_connector_line_color: Color,
    preview_connector_line_color: Color,
    default_port_color: Color,
    selected_connector_port_color: Color,
    active_new_line_start_port_color: Color,

    last_click_time: Option<Instant>,
    last_click_pos: Option<Vec2>, 
    selected_shape_index: Option<usize>,
    dragged_shape_index: Option<usize>,
    drag_offset: Option<Vec2>, 
    editing_shape_index: Option<usize>,
    current_input_text: String,

    connections: Vec<UserConnection>, 
    selected_connector_index: Option<usize>, 

    drawing_new_line: bool,
    new_line_start_info: Option<(usize, bool)>, 
    new_line_preview_end_pos: Option<Vec2>,
}

impl AppState {
    fn new(_ctx: &mut Context, app_config: &AppConfig) -> GameResult<AppState> {
        let shape_config = &app_config.shape;
        let colors_config = app_config.colors.clone().unwrap_or_default(); 

        // Shape base color
        let default_shape_color = Color::from_rgb(
            shape_config.base_color_rgb[0],
            shape_config.base_color_rgb[1],
            shape_config.base_color_rgb[2],
        );

        // Shape selection outline color
        let selection_outline_color = shape_config.selection_outline_color_rgb
            .map_or(Color::from_rgb(255, 255, 0), |rgb| Color::from_rgb(rgb[0], rgb[1], rgb[2])); // Default Yellow

        let selection_outline_width = shape_config.selection_outline_width.unwrap_or(2.0);

        let ui_scale = match app_config.window.ui_scale_factor {
            Some(factor) if factor > 0.0 => factor,
            Some(_) => {
                println!("Warning: Invalid ui_scale_factor in config.toml. Must be > 0. Defaulting to 1.0.");
                1.0
            }
            None => 1.0,
        };
        println!("Using UI Scale Factor: {}", ui_scale);

        // Load other colors or use defaults
        let connector_line_color = colors_config.connector_line_rgb
            .map_or(Color::WHITE, |rgb| Color::from_rgb(rgb[0], rgb[1], rgb[2]));
        let selected_connector_line_color = colors_config.selected_connector_line_rgb
            .map_or(Color::CYAN, |rgb| Color::from_rgb(rgb[0], rgb[1], rgb[2]));
        
        // Preview line color: use RGB from config, hardcode alpha
        let preview_connector_line_color_rgb = colors_config.preview_connector_line_rgb
            .unwrap_or([204, 204, 204]); // Default light gray RGB
        let preview_connector_line_color = Color::from_rgba(
            preview_connector_line_color_rgb[0],
            preview_connector_line_color_rgb[1],
            preview_connector_line_color_rgb[2],
            178, // Alpha for ~0.7 opacity (0-255 range)
        );

        let default_port_color = colors_config.default_port_rgb
            .map_or(Color::WHITE, |rgb| Color::from_rgb(rgb[0], rgb[1], rgb[2]));
        let selected_connector_port_color = colors_config.selected_connector_port_rgb
            .map_or(Color::CYAN, |rgb| Color::from_rgb(rgb[0], rgb[1], rgb[2]));
        let active_new_line_start_port_color = colors_config.active_new_line_start_port_rgb
            .map_or(Color::from_rgb(50, 205, 50), |rgb| Color::from_rgb(rgb[0], rgb[1], rgb[2]));


        Ok(AppState {
            live_mouse_pos: Vec2::new(0.0, 0.0),
            clicked_shapes: Vec::new(),
            default_shape_color, // Use loaded/defaulted shape color
            default_shape_width: shape_config.width,
            default_shape_height: shape_config.height,
            default_shape_corner_radius: shape_config.corner_radius,
            selection_outline_color,
            selection_outline_width,
            ui_scale, 
            connector_line_color,
            selected_connector_line_color,
            preview_connector_line_color,
            default_port_color,
            selected_connector_port_color,
            active_new_line_start_port_color,
            last_click_time: None,
            last_click_pos: None,
            selected_shape_index: None,
            dragged_shape_index: None,
            drag_offset: None,
            editing_shape_index: None,
            current_input_text: String::new(),
            connections: Vec::new(), 
            selected_connector_index: None, 
            drawing_new_line: false,
            new_line_start_info: None,
            new_line_preview_end_pos: None,
        })
    }

    // Helper to get port coordinates
    fn get_port_point(&self, shape_index: usize, is_outgoing_port: bool) -> Option<Vec2> {
        if shape_index < self.clicked_shapes.len() {
            let shape_data = &self.clicked_shapes[shape_index];
            let x_base = shape_data.center_position.x - self.default_shape_width / 2.0;
            let y_base = shape_data.center_position.y;
            if is_outgoing_port { // Bottom-left port
                Some(Vec2::new(x_base + CONNECTOR_POINT_HORIZONTAL_OFFSET, y_base + self.default_shape_height / 2.0))
            } else { // Top-left port
                Some(Vec2::new(x_base + CONNECTOR_POINT_HORIZONTAL_OFFSET, y_base - self.default_shape_height / 2.0))
            }
        } else {
            None
        }
    }
}

// Helper function to get a point on a cubic Bezier curve
fn get_point_on_cubic_bezier(p0: LyonPoint, p1: LyonPoint, p2: LyonPoint, p3: LyonPoint, t: f32) -> LyonPoint {
    let t_inv = 1.0 - t;
    let t_inv_sq = t_inv * t_inv;
    let t_inv_cub = t_inv_sq * t_inv;
    let t_sq = t * t;
    let t_cub = t_sq * t;
    let x = t_inv_cub * p0.x + 3.0 * t_inv_sq * t * p1.x + 3.0 * t_inv * t_sq * p2.x + t_cub * p3.x;
    let y = t_inv_cub * p0.y + 3.0 * t_inv_sq * t * p1.y + 3.0 * t_inv * t_sq * p2.y + t_cub * p3.y;
    LyonPoint::new(x, y)
}


// --- EventHandler Implementation ---
impl EventHandler<ggez::GameError> for AppState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        if self.drawing_new_line {
            self.new_line_preview_end_pos = Some(self.live_mouse_pos);
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::from_rgb(30, 30, 40));

        let (physical_width, physical_height) = ctx.gfx.drawable_size();
        let logical_width = physical_width / self.ui_scale;
        let logical_height = physical_height / self.ui_scale;
        canvas.set_screen_coordinates(Rect::new(0.0, 0.0, logical_width, logical_height));

        // --- Draw Existing Connector Lines ---
        for (conn_idx, connection) in self.connections.iter().enumerate() {
            if let (Some(start_point_ggez), Some(end_point_ggez)) = (
                self.get_port_point(connection.from_shape_index, true), 
                self.get_port_point(connection.to_shape_index, false)    
            ) {
                let start_point_lyon = LyonPoint::new(start_point_ggez.x, start_point_ggez.y);
                let end_point_lyon = LyonPoint::new(end_point_ggez.x, end_point_ggez.y);

                let direction_multiplier = if end_point_lyon.x > start_point_lyon.x { 1.0 } else { -1.0 };
                let cp1 = LyonPoint::new(start_point_lyon.x + CONNECTOR_CURVE_OFFSET * direction_multiplier, start_point_lyon.y);
                let cp2 = LyonPoint::new(end_point_lyon.x - CONNECTOR_CURVE_OFFSET * direction_multiplier, end_point_lyon.y);

                let mut path_builder = LyonPathBuilder::new();
                path_builder.begin(start_point_lyon);
                path_builder.cubic_bezier_to(cp1, cp2, end_point_lyon);
                path_builder.end(false); 
                let lyon_path = path_builder.build();

                let current_line_color = if self.selected_connector_index == Some(conn_idx) {
                    self.selected_connector_line_color
                } else {
                    self.connector_line_color
                };
                
                let mut geometry: VertexBuffers<Vertex, u32> = VertexBuffers::new();
                let mut stroke_tess = StrokeTessellator::new();
                let stroke_options = StrokeOptions::default().with_line_width(CONNECTOR_LINE_WIDTH);
                let line_color_arr = [
                    current_line_color.r, current_line_color.g, current_line_color.b, current_line_color.a,
                ];

                stroke_tess.tessellate_path( &lyon_path, &stroke_options,
                    &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                        Vertex { position: [vertex.position().x, vertex.position().y], uv: [0.0, 0.0], color: line_color_arr, }
                    }),
                ).unwrap_or_else(|e| {println!("Lyon tessellation error: {:?}", e);});

                if !geometry.vertices.is_empty() && !geometry.indices.is_empty() {
                    let mesh_data = MeshData { vertices: &geometry.vertices, indices: &geometry.indices };
                    let line_mesh = Mesh::from_data(ctx, mesh_data); 
                    canvas.draw(&line_mesh, graphics::DrawParam::default());
                }
            }
        }
        
        // --- Draw Preview Connector Line ---
        if self.drawing_new_line {
            if let (Some((start_shape_idx, start_is_outgoing)), Some(preview_end_pos)) = (self.new_line_start_info, self.new_line_preview_end_pos) {
                if let Some(start_pos) = self.get_port_point(start_shape_idx, start_is_outgoing) {
                     let line_preview_mesh = Mesh::new_line(ctx, &[start_pos, preview_end_pos], CONNECTOR_LINE_WIDTH / 2.0, self.preview_connector_line_color)?;
                     canvas.draw(&line_preview_mesh, graphics::DrawParam::default());
                }
            }
        }


        // --- Draw Shapes, Outlines, Text, and Ports on Shapes ---
        for (index, shape_data) in self.clicked_shapes.iter().enumerate() {
            let rect = Rect::new(
                shape_data.center_position.x - self.default_shape_width / 2.0,
                shape_data.center_position.y - self.default_shape_height / 2.0,
                self.default_shape_width,
                self.default_shape_height,
            );
            let rounded_rect_mesh = Mesh::new_rounded_rectangle(ctx, DrawMode::fill(), rect, self.default_shape_corner_radius, self.default_shape_color)?;
            canvas.draw(&rounded_rect_mesh, graphics::DrawParam::default());

            // Determine port colors and radii
            let mut outgoing_port_color = self.default_port_color;
            let mut incoming_port_color = self.default_port_color;
            let mut outgoing_port_radius = PORT_DRAW_RADIUS_DEFAULT;
            let mut incoming_port_radius = PORT_DRAW_RADIUS_DEFAULT;

            if let Some(conn_idx) = self.selected_connector_index {
                if conn_idx < self.connections.len() {
                    let selected_conn = &self.connections[conn_idx];
                    if selected_conn.from_shape_index == index { outgoing_port_color = self.selected_connector_port_color; }
                    if selected_conn.to_shape_index == index { incoming_port_color = self.selected_connector_port_color; }
                }
            }
            if let Some((start_idx, is_out)) = self.new_line_start_info {
                if start_idx == index {
                    if is_out { outgoing_port_color = self.active_new_line_start_port_color; }
                    else { incoming_port_color = self.active_new_line_start_port_color; }
                }
            }

            // Check for hover on outgoing port
            if let Some(outgoing_point_ggez) = self.get_port_point(index, true) {
                if self.live_mouse_pos.distance(outgoing_point_ggez) <= PORT_HOVER_DETECT_DISTANCE {
                    outgoing_port_radius = PORT_DRAW_RADIUS_HOVER;
                }
                let outgoing_port_mesh = Mesh::new_circle(ctx, DrawMode::fill(), outgoing_point_ggez, outgoing_port_radius, 0.1, outgoing_port_color)?;
                canvas.draw(&outgoing_port_mesh, graphics::DrawParam::default());
            }

            // Check for hover on incoming port
            if let Some(incoming_point_ggez) = self.get_port_point(index, false) {
                 if self.live_mouse_pos.distance(incoming_point_ggez) <= PORT_HOVER_DETECT_DISTANCE {
                    incoming_port_radius = PORT_DRAW_RADIUS_HOVER;
                }
                let incoming_port_mesh = Mesh::new_circle(ctx, DrawMode::fill(), incoming_point_ggez, incoming_port_radius, 0.1, incoming_port_color)?;
                canvas.draw(&incoming_port_mesh, graphics::DrawParam::default());
            }


            if self.selected_shape_index == Some(index) && self.editing_shape_index != Some(index) {
                let center_x = rect.x + rect.w / 2.0;
                let center_y = rect.y + rect.h / 2.0;
                let outline_w = rect.w * 1.05;
                let outline_h = rect.h * 1.05;
                let outline_bounds = Rect::new(center_x - outline_w / 2.0, center_y - outline_h / 2.0, outline_w, outline_h);
                let outline_rect_mesh = Mesh::new_rounded_rectangle(ctx, DrawMode::stroke(self.selection_outline_width), outline_bounds, self.default_shape_corner_radius * 1.05, self.selection_outline_color)?;
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
            "Mouse: {:.0}, {:.0} | Shapes: {} {}{}{}{}", 
            self.live_mouse_pos.x, 
            self.live_mouse_pos.y,
            self.clicked_shapes.len(),
            if self.editing_shape_index.is_some() { "[EDITING SHAPE]" } else { "" },
            if self.selected_shape_index.is_some() && self.editing_shape_index.is_none() { "[SHAPE SELECTED]" } else { "" },
            if self.selected_connector_index.is_some() { "[CONN SELECTED]" } else { "" },
            if self.drawing_new_line { "[DRAWING LINE]" } else { "" }
        );
        let mut text_display = graphics::Text::new(status_text);
        text_display.set_scale(20.0); 
        canvas.draw(&text_display, graphics::DrawParam::default().dest(Vec2::new(10.0, 10.0)).color(Color::WHITE));
        
        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) -> GameResult {
        let logical_x = x / self.ui_scale;
        let logical_y = y / self.ui_scale;
        let current_click_pos = Vec2::new(logical_x, logical_y);
        let current_click_time = Instant::now();

        if button == MouseButton::Left {
            // --- Priority 1: Completing a new line ---
            if self.drawing_new_line {
                let mut connected_to_target = false;
                if let Some((start_shape_idx, _start_is_outgoing)) = self.new_line_start_info {
                    for (target_idx, _target_shape_data) in self.clicked_shapes.iter().enumerate() {
                        if target_idx == start_shape_idx { continue; } 

                        if let Some(target_incoming_pos) = self.get_port_point(target_idx, false) { 
                            if current_click_pos.distance(target_incoming_pos) <= PORT_CLICK_RADIUS { 
                                let new_connection = UserConnection { from_shape_index: start_shape_idx, to_shape_index: target_idx };
                                if !self.connections.contains(&new_connection) { self.connections.push(new_connection); }
                                connected_to_target = true; break;
                            }
                        }
                        if let Some(target_outgoing_pos) = self.get_port_point(target_idx, true) { 
                             if current_click_pos.distance(target_outgoing_pos) <= PORT_CLICK_RADIUS { 
                                let new_connection = UserConnection { from_shape_index: start_shape_idx, to_shape_index: target_idx };
                                 if !self.connections.contains(&new_connection) { self.connections.push(new_connection); }
                                connected_to_target = true; break;
                            }
                        }
                    }
                }
                self.drawing_new_line = false; self.new_line_start_info = None; self.new_line_preview_end_pos = None;
                if !connected_to_target { println!("New line drawing cancelled."); }
                return Ok(());
            }

            // --- Priority 2: Interacting with a shape body ---
            let mut clicked_on_shape_body_details: Option<(usize, Vec2)> = None;
            for (index, shape_data) in self.clicked_shapes.iter().enumerate().rev() {
                let shape_rect = Rect::new(
                    shape_data.center_position.x - self.default_shape_width / 2.0,
                    shape_data.center_position.y - self.default_shape_height / 2.0,
                    self.default_shape_width, self.default_shape_height);
                if shape_rect.contains(current_click_pos) {
                    clicked_on_shape_body_details = Some((index, shape_data.center_position));
                    break;
                }
            }

            if let Some((clicked_idx, clicked_shape_center)) = clicked_on_shape_body_details {
                self.selected_connector_index = None;
                if self.editing_shape_index.is_some() && self.editing_shape_index != Some(clicked_idx) {
                    if let Some(editing_idx_val) = self.editing_shape_index.take() {
                        self.clicked_shapes[editing_idx_val].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                        self.current_input_text.clear();
                    }
                }
                self.selected_shape_index = Some(clicked_idx);
                let mut is_double_click_for_edit = false;
                if let (Some(last_time), Some(last_pos_val)) = (self.last_click_time, self.last_click_pos) {
                    if current_click_time.duration_since(last_time).as_millis() <= DOUBLE_CLICK_MAX_DELAY_MS && current_click_pos.distance(last_pos_val) <= DOUBLE_CLICK_MAX_DISTANCE {
                        is_double_click_for_edit = true;
                    }
                }
                if is_double_click_for_edit {
                    self.editing_shape_index = Some(clicked_idx);
                    self.current_input_text = self.clicked_shapes[clicked_idx].text.clone().unwrap_or_default();
                    self.dragged_shape_index = None; self.last_click_time = None; self.last_click_pos = None;
                } else {
                    self.dragged_shape_index = Some(clicked_idx);
                    self.drag_offset = Some(clicked_shape_center - current_click_pos);
                    self.last_click_time = Some(current_click_time); self.last_click_pos = Some(current_click_pos);
                }
                return Ok(());
            }
            
            // --- Priority 3: Starting a new line from a port ---
            for (index, _shape_data) in self.clicked_shapes.iter().enumerate() {
                if let Some(outgoing_pos) = self.get_port_point(index, true) {
                    if current_click_pos.distance(outgoing_pos) <= PORT_CLICK_RADIUS { 
                        self.drawing_new_line = true; self.new_line_start_info = Some((index, true));
                        self.selected_shape_index = None; self.selected_connector_index = None;
                        self.last_click_time = None; self.last_click_pos = None;
                        println!("Starting new line from shape {} (outgoing port).", index); return Ok(());
                    }
                }
                if let Some(incoming_pos) = self.get_port_point(index, false) {
                    if current_click_pos.distance(incoming_pos) <= PORT_CLICK_RADIUS { 
                        self.drawing_new_line = true; self.new_line_start_info = Some((index, false));
                        self.selected_shape_index = None; self.selected_connector_index = None;
                        self.last_click_time = None; self.last_click_pos = None;
                        println!("Starting new line from shape {} (incoming port).", index); return Ok(());
                    }
                }
            }

            // --- Priority 4: Selecting an existing connector line ---
            let mut clicked_on_existing_connector_idx: Option<usize> = None;
            for (conn_idx, connection) in self.connections.iter().enumerate() {
                 if let (Some(start_point_ggez), Some(end_point_ggez)) = (
                    self.get_port_point(connection.from_shape_index, true),
                    self.get_port_point(connection.to_shape_index, false)
                ) {
                    let p0 = LyonPoint::new(start_point_ggez.x, start_point_ggez.y);
                    let p3 = LyonPoint::new(end_point_ggez.x, end_point_ggez.y);
                    let dir_mult = if p3.x > p0.x { 1.0 } else { -1.0 };
                    let p1 = LyonPoint::new(p0.x + CONNECTOR_CURVE_OFFSET * dir_mult, p0.y);
                    let p2 = LyonPoint::new(p3.x - CONNECTOR_CURVE_OFFSET * dir_mult, p3.y);
                    for j in 0..=CONNECTOR_SAMPLE_POINTS {
                        let t = j as f32 / CONNECTOR_SAMPLE_POINTS as f32;
                        let curve_point = get_point_on_cubic_bezier(p0, p1, p2, p3, t);
                        if current_click_pos.distance(Vec2::new(curve_point.x, curve_point.y)) <= CONNECTOR_SELECTION_RADIUS {
                            clicked_on_existing_connector_idx = Some(conn_idx); break;
                        }
                    }
                }
                if clicked_on_existing_connector_idx.is_some() { break; }
            }

            if let Some(conn_idx) = clicked_on_existing_connector_idx {
                self.selected_connector_index = Some(conn_idx);
                self.selected_shape_index = None; 
                self.editing_shape_index = None; 
                if let Some(editing_idx_val) = self.editing_shape_index.take() {
                     self.clicked_shapes[editing_idx_val].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                     self.current_input_text.clear();
                }
                println!("Connector {} selected.", conn_idx);
                self.last_click_time = Some(current_click_time); self.last_click_pos = Some(current_click_pos);
                return Ok(());
            }

            // --- Priority 5: Clicking on empty space ---
            if let Some(editing_idx_val) = self.editing_shape_index.take() { 
                self.clicked_shapes[editing_idx_val].text = if self.current_input_text.is_empty() { None } else { Some(self.current_input_text.clone()) };
                self.current_input_text.clear();
            }
            self.selected_shape_index = None; self.dragged_shape_index = None; self.selected_connector_index = None;

            let mut is_double_click_for_create = false;
            if let (Some(last_time), Some(last_pos_val)) = (self.last_click_time, self.last_click_pos) {
                if current_click_time.duration_since(last_time).as_millis() <= DOUBLE_CLICK_MAX_DELAY_MS && current_click_pos.distance(last_pos_val) <= DOUBLE_CLICK_MAX_DISTANCE {
                    is_double_click_for_create = true;
                }
            }
            if is_double_click_for_create {
                self.clicked_shapes.push(ShapeData { center_position: current_click_pos, text: None });
                let new_idx = self.clicked_shapes.len() - 1;
                self.selected_shape_index = Some(new_idx); self.editing_shape_index = Some(new_idx);
                self.current_input_text.clear();
                self.last_click_time = None; self.last_click_pos = None;
            } else {
                self.last_click_time = Some(current_click_time); self.last_click_pos = Some(current_click_pos);
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
        let logical_x = x / self.ui_scale;
        let logical_y = y / self.ui_scale;
        self.live_mouse_pos = Vec2::new(logical_x, logical_y);
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
            if self.drawing_new_line && keycode == KeyCode::Escape && !repeated {
                self.drawing_new_line = false; self.new_line_start_info = None; self.new_line_preview_end_pos = None;
                println!("New line drawing cancelled by Escape.");
                return Ok(());
            }

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
                    _ => { if repeated { return Ok(()); } } 
                }
            } else if let Some(index_to_delete) = self.selected_shape_index { 
                if (keycode == KeyCode::Delete || keycode == KeyCode::Back) && !repeated { 
                    let deleted_shape_idx = index_to_delete;
                    self.clicked_shapes.remove(deleted_shape_idx);
                    
                    let mut new_connections = Vec::new();
                    for conn in self.connections.iter() {
                        if conn.from_shape_index == deleted_shape_idx || conn.to_shape_index == deleted_shape_idx {
                            continue; 
                        }
                        let mut new_conn = conn.clone();
                        if conn.from_shape_index > deleted_shape_idx { new_conn.from_shape_index -= 1; }
                        if conn.to_shape_index > deleted_shape_idx { new_conn.to_shape_index -= 1; }
                        new_connections.push(new_conn);
                    }
                    self.connections = new_connections;

                    self.selected_shape_index = None;
                    self.dragged_shape_index = None; 
                    self.editing_shape_index = None; 
                    self.selected_connector_index = None; 
                    self.last_click_time = None; 
                    self.last_click_pos = None;
                    println!("Shape {} deleted, connections updated.", deleted_shape_idx);
                }
            } else if let Some(connector_idx_to_delete) = self.selected_connector_index { 
                if (keycode == KeyCode::Delete || keycode == KeyCode::Back) && !repeated { 
                    if connector_idx_to_delete < self.connections.len() {
                        self.connections.remove(connector_idx_to_delete);
                        println!("Connector {} deleted.", connector_idx_to_delete);
                    }
                    self.selected_connector_index = None;
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
            title: "Rust: Shapes - Configurable Colors (Default)".to_string(), 
            msaa_level: None, 
            ui_scale_factor: None, 
        },
        shape: ShapeConfig {
            width: 120.0,
            height: 70.0,
            corner_radius: 10.0,
            base_color_rgb: [100, 200, 255], // Default shape base color
            selection_outline_color_rgb: None, // Will default to Yellow in AppState
            selection_outline_width: None,
        },
        colors: None, 
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
    let app_config = load_config(); 

    let msaa = match app_config.window.msaa_level {
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


    let (mut ctx, event_loop) = ContextBuilder::new("shapes_app_configurable_colors", "YourName")
        .window_setup(
            WindowSetup::default()
                .title(&app_config.window.title)
                .samples(msaa) 
        )
        .window_mode(
            WindowMode::default()
                .dimensions(app_config.window.width, app_config.window.height) 
                .resizable(true)
        )
        .build()?;
    
    let app_state = AppState::new(&mut ctx, &app_config)?;
    
    event::run(ctx, event_loop, app_state)
}

