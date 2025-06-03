// main.rs

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, MouseButton};
use ggez::glam::Vec2; // For 2D vectors (coordinates)
//use ggez::graphics::{self, Color, DrawMode, Mesh, Font}; // Added Font for clarity, though not strictly needed for default
use ggez::graphics::{self, Color, DrawMode, Mesh }; // Added Font for clarity, though not strictly needed for default
use ggez::{Context, ContextBuilder, GameResult};

// Constants for window size and title
const SCREEN_WIDTH: f32 = 800.0;
const SCREEN_HEIGHT: f32 = 600.0;
const WINDOW_TITLE: &str = "Rust Visual App with Mouse Input";

// Structure to hold the application's state
struct AppState {
    mouse_pos: Vec2,         // Current mouse position
    circle_color: Color,     // Color of the circle
    circle_radius: f32,      // Radius of the circle
    is_mouse_pressed: bool,  // Tracks if the primary mouse button is pressed
}

impl AppState {
    // Constructor for our application state
    fn new(_ctx: &mut Context) -> GameResult<AppState> {
        Ok(AppState {
            mouse_pos: Vec2::new(SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0), // Start in center
            circle_color: Color::BLUE, // Initial color
            circle_radius: 50.0,
            is_mouse_pressed: false,
        })
    }
}

// Implementing the EventHandler trait for our AppState
impl EventHandler<ggez::GameError> for AppState {
    // update() is called every frame before drawing.
    // It's where you put your game logic.
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        // In this simple example, most logic is event-driven,
        // but you could update animations or physics here.
        Ok(())
    }

    // draw() is called every frame to render graphics.
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 1. Clear the screen with a background color
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::from_rgb(30, 30, 40));

        // 2. Create a mesh for the circle
        // A mesh is a collection of points, lines, or triangles that form a shape.
        let circle_mesh = Mesh::new_circle(
            ctx,
            DrawMode::fill(),    // Draw a filled circle
            self.mouse_pos,      // Position the circle at the current mouse coordinates
            self.circle_radius,  // Radius of the circle
            0.1,                 // Tolerance (detail) of the circle's geometry
            self.circle_color,   // Color of the circle
        )?;

        // 3. Draw the circle mesh onto the canvas
        canvas.draw(&circle_mesh, graphics::DrawParam::default());

        // 4. Display mouse coordinates as text (optional)
        let coords_text_string = format!("Mouse: {:.0}, {:.0}", self.mouse_pos.x, self.mouse_pos.y);
        
        // Create a Text object. This will use ggez's default font (LiberationMono).
        let mut text_display = graphics::Text::new(coords_text_string);
        
        // Set the scale of the text.
        // The set_scale method takes &mut self, so we call it on our mutable text_display.
        text_display.set_scale(24.0);
        // If you had a specific `Font` object loaded, you would call:
        // let my_font = graphics::Font::new(ctx, "/path/to/your/font.ttf")?;
        // text_display.set_font(my_font.clone()); // Font might need to be cloned if used elsewhere
        // text_display.set_font_scale(24.0); // older API, use set_scale for uniform scaling

        // Draw the text in the top-left corner.
        // We pass an immutable reference `&text_display` which is `&Text`.
        // `Text` implements `Drawable`.
        canvas.draw(
            &text_display, // Pass &Text
            graphics::DrawParam::default()
                .dest(Vec2::new(10.0, 10.0))
                .color(Color::WHITE),
        );

        // 5. Present the canvas to the screen
        canvas.finish(ctx)?;

        Ok(())
    }

    // Called when a mouse button is pressed
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        if button == MouseButton::Left {
            println!("Left mouse button pressed at: ({}, {})", x, y);
            self.is_mouse_pressed = true;
            self.circle_color = Color::RED; // Change color on click
        }
        Ok(())
    }

    // Called when a mouse button is released
    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        if button == MouseButton::Left {
            println!("Left mouse button released at: ({}, {})", x, y);
            self.is_mouse_pressed = false;
            self.circle_color = Color::BLUE; // Change color back
        }
        Ok(())
    }

    // Called when the mouse is moved
    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) -> GameResult {
        // Update the stored mouse position
        self.mouse_pos = Vec2::new(x, y);
        Ok(())
    }

}

// The main function: entry point of the application
pub fn main() -> GameResult {
    let (mut ctx, event_loop) = ContextBuilder::new("mouse_visual_app", "YourName")
        .window_setup(WindowSetup::default().title(WINDOW_TITLE))
        .window_mode(
            WindowMode::default()
                .dimensions(SCREEN_WIDTH, SCREEN_HEIGHT)
                .resizable(true),
        )
        .build()?;

    let app_state = AppState::new(&mut ctx)?;
    event::run(ctx, event_loop, app_state)
}

