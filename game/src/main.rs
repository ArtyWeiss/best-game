use platform::{
    vulkan::{self, VulkanContext},
    window::{self, Window},
};

fn main() {
    let mut window = Window::new("Best Game".to_string(), 400, 400);
    let mut vulkan_context = VulkanContext::new(400, 400);

    while window.exists {
        window::update_window(&mut window);
        vulkan::update_context(&mut vulkan_context, &window);
        vulkan::update_pass(&mut vulkan_context);
        vulkan::draw_frame(&mut vulkan_context);
    }
}
