use platform::window::{self, Window};

fn main() {
    println!("Start");
    let mut window = Window::default();
    window.width = 100;
    window.height = 100;
    while !window.exit {
        window::update_window(&mut window);
    }
    println!("Finish");
}
