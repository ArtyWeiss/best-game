use ash::vk;

use super::InternalContext;

pub struct PassConfiguration {
    pub clear_color: [f32; 4],
}

pub struct Pass {
    pub raw: vk::RenderPass,
    pub clear_value: vk::ClearValue,
    pub framebuffers: Vec<vk::Framebuffer>,
}

pub fn create_pass(internal: &mut InternalContext, pass_config: &PassConfiguration) -> Pass {
    let attachments = [vk::AttachmentDescription {
        format: internal.surface_format.format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        ..Default::default()
    }];
    let color_attachment_refs = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
    let subpass_deps = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        ..Default::default()
    }];
    let subpasses = [vk::SubpassDescription::default()
        .color_attachments(&color_attachment_refs)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)];

    let create_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&subpass_deps);
    let raw = unsafe {
        internal.device.create_render_pass(&create_info, None).expect("Cant create render pass")
    };

    let framebuffers = create_framebuffers(
        &internal.device,
        &internal.swapchain_image_views,
        raw,
        internal.surface_resolution.width,
        internal.surface_resolution.height,
    );

    Pass {
        raw,
        clear_value: vk::ClearValue {
            color: vk::ClearColorValue { float32: pass_config.clear_color },
        },
        framebuffers,
    }
}

pub fn begin_pass(context: &InternalContext, pass: &Pass, present_index: u32) {
    unsafe {
        let clear_values = [pass.clear_value];
        let begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(pass.raw)
            .clear_values(&clear_values)
            .render_area(context.surface_resolution.into())
            .framebuffer(pass.framebuffers[present_index as usize]);
        context.device.cmd_begin_render_pass(
            context.command_buffer,
            &begin_info,
            vk::SubpassContents::INLINE,
        );
    }
}

pub fn end_pass(context: &InternalContext) {
    unsafe {
        context.device.cmd_end_render_pass(context.command_buffer);
    }
}

pub fn destroy_pass(context: &InternalContext, pass: &mut Pass) {
    unsafe {
        for fb in pass.framebuffers.iter() {
            context.device.destroy_framebuffer(*fb, None);
        }
        context.device.destroy_render_pass(pass.raw, None);
    }
}

pub fn create_framebuffers(
    device: &ash::Device,
    swapchain_image_views: &[vk::ImageView],
    pass: vk::RenderPass,
    width: u32,
    height: u32,
) -> Vec<vk::Framebuffer> {
    swapchain_image_views
        .iter()
        .map(|&image_view| {
            let attachments = [image_view];
            let create_info = vk::FramebufferCreateInfo::default()
                .render_pass(pass)
                .attachments(&attachments)
                .width(width)
                .height(height)
                .layers(1);

            unsafe {
                device.create_framebuffer(&create_info, None).expect("Cant create framebuffer")
            }
        })
        .collect()
}
