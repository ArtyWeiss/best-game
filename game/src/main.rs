use platform::window;

fn main() {
    println!("Start");
    let mut window = window::create("Best-Game", 100, 100);
    loop {
        window::update(&mut window);
        if window.exit {
            break;
        }
    }
    println!("Finish");
}
