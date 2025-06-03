// main.rs

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, MouseButton};
use ggez::glam::Vec2; // For 2D vectors (coordinates)
use ggez::graphics::{self, Color, DrawMode, Mesh};
use ggez::{Context, ContextBuilder, GameResult};

// Constants for window size and title
const SCREEN_WIDTH: f32 = 800.0;
const SCREEN_HEIGHT: f32 = 600.0;
const WINDOW_TITLE: &str = "Rust: Click to Add Multiple Circles";

// Structure to hold the application's state
struct AppState {
    live_mouse_pos: Vec2,             // Current live mouse position for text display
    clicked_circles_positions: Vec<Vec2>, // Stores the center positions of all clicked circles
    circle_color: Color,              // Color for all circles
    circle_radius: f32,               // Radius for all circles
}

impl AppState {
    // Constructor for our application state
    fn new(_ctx: &mut Context) -> GameResult<AppState> {
        Ok(AppState {
            live_mouse_pos: Vec2::new(0.0, 0.0), // Initialize live mouse position
            clicked_circles_positions: Vec::new(), // Start with an empty list of circles
            circle_color: Color::from_rgb(100, 200, 255), // A nice light blue for the circles
            circle_radius: 30.0,
        })
    }
}

// Implementing the EventHandler trait for our AppState
impl EventHandler<ggez::GameError> for AppState {
    // update() is called every frame before drawing.
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        // Game logic can go here if needed for animations or other state changes.
        Ok(())
    }

    // draw() is called every frame to render graphics.
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 1. Clear the screen with a background color
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::from_rgb(30, 30, 40));

        // 2. Draw all stored circles
        for &pos in &self.clicked_circles_positions { // Iterate over the stored positions
            let circle_mesh = Mesh::new_circle(
                ctx,
                DrawMode::fill(),
                pos, // Draw at the stored click position
                self.circle_radius,
                0.1, // Tolerance for circle smoothness
                self.circle_color,
            )?;
            canvas.draw(&circle_mesh, graphics::DrawParam::default());
        }

        // 3. Display live mouse coordinates as text
        let coords_text_string = format!(
            "Mouse: {:.0}, {:.0} | Circles: {}",
            self.live_mouse_pos.x,
            self.live_mouse_pos.y,
            self.clicked_circles_positions.len() // Display number of circles
        );
        let mut text_display = graphics::Text::new(coords_text_string);
        text_display.set_scale(20.0); // Slightly smaller text to fit more info

        canvas.draw(
            &text_display,
            graphics::DrawParam::default()
                .dest(Vec2::new(10.0, 10.0))
                .color(Color::WHITE),
        );

        // 4. Present the canvas to the screen
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
            let click_pos = Vec2::new(x, y);
            println!(
                "Left mouse button pressed at: ({}, {}) - New circle added.",
                click_pos.x, click_pos.y
            );
            // Add the new click position to our list of circles
            self.clicked_circles_positions.push(click_pos);
        }
        // You could add a way to clear circles, e.g., on right-click:
        // if button == MouseButton::Right {
        //     println!("Right mouse button pressed - Clearing circles.");
        //     self.clicked_circles_positions.clear();
        // }
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
            // This event is less critical now for drawing, but good for logging or other actions
            println!("Left mouse button released at: ({}, {})", x, y);
        }
        Ok(())
    }

    // Called when the mouse is moved
    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        // Update the live mouse position for the text display
        self.live_mouse_pos = Vec2::new(x, y);
        Ok(())
    }
}

// The main function: entry point of the application
pub fn main() -> GameResult {
    let (mut ctx, event_loop) = ContextBuilder::new("mouse_add_multiple_circles", "YourName")
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
