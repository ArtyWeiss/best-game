use platform::window::{self, Callbacks, Window};
use renderer::Renderer;

static mut RENDERER: Renderer = Renderer {
    width: 0,
    height: 0,
};

fn main() {
    println!("Start");
    let mut window = Window::default();
    let mut callbacks = Callbacks {
        resize,
        close,
        exit,
        repaint,
    };
    window.width = 100;
    window.height = 100;
    //    while !window.exit {
    window::update_window(&mut window, &mut callbacks);
    //    }
    println!("Finish");
}

fn resize(w: u32, h: u32) {
    unsafe {
        let renderer = &mut RENDERER;
        renderer.width = w;
        renderer.height = h;
    }
    println!("resize");
}

fn close() {
    println!("close");
}

fn exit() {}

fn repaint() {
    println!("repaint");
}
