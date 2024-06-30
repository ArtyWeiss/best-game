#[derive(Default)]
pub struct Renderer {
    pub width: u32,
    pub height: u32,
}

impl Renderer {
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}
