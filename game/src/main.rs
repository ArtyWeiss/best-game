use platform::window::{self, Window};

fn main() {
    let mut window = Window::new("Best Game".to_string(), 400, 400);
    while window.exists {
        window::update_window(&mut window);
        if !window.events.is_empty() {
            println!("{:#?}", window.events);
        }
    }
}
