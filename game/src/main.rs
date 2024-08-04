use platform::{vulkan::*, window::*};

fn main() {
    let mut window = Window::new("Best Game".to_string(), 400, 400);
    let mut vulkan_context = VulkanContext::new(400, 400, true);

    while window.exists {
//        platform::utils::trace(format!("w{}h{}", window.width, window.height));
        update_window(&mut window);
        update_context(&mut vulkan_context, &window);
        update_pass(&mut vulkan_context);
        draw_frame(&mut vulkan_context);
    }
}
