use platform::{vulkan::*, window::*};

fn main() {
    let mut window = Window::new("Лучшая игра".to_string(), 400, 400);
    let mut vulkan_context = VulkanContext::new(400, 400, true);

    while window.exists {
        update_window(&mut window);
        update_context(&mut vulkan_context, &window);
        update_pass(&mut vulkan_context);

        begin_frame(&mut vulkan_context);
        draw_pass(&mut vulkan_context);
        end_frame(&mut vulkan_context);
    }
}
