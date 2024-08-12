use platform::{utils, vulkan::*, window::*};

fn main() {
    let mut window = Window::new("Лучшая игра".to_string(), 400, 400);
    let mut vulkan_context = VulkanContext::new(400, 400, true);
    let mut pipeline = None;

    while window.exists {
        update_window(&mut window);
        update_context(&mut vulkan_context, &window);
        update_pass(&mut vulkan_context);

        if pipeline.is_none() {
            let pipeline_config = PipelineConfig {
                vertext_shader_source: include_bytes!("../assets/compiled/test.vert.spv"),
                fragment_shader_source: include_bytes!("../assets/compiled/test.frag.spv"),
            };
            pipeline = create_pipeline(&mut vulkan_context, pipeline_config).ok();
            utils::trace(format!("Pipeline created: {:?}", pipeline));
        }

        begin_frame(&mut vulkan_context);
        draw_pass(&mut vulkan_context);
        end_frame(&mut vulkan_context);
    }
}
