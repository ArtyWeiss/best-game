use platform::{
    vulkan::VulkanContext,
    window::{self, Window},
};

fn main() {
    let mut window = Window::new("Best Game".to_string(), 400, 400);
    let mut vulkan_context = VulkanContext::new(&window);

    while window.exists {
        window::update_window(&mut window);
    }

    vulkan_context.destroy();
}
