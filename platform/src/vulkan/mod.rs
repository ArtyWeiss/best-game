use ash::vk;

use crate::utils;
use crate::window::{Window, WindowEvent};

use context::*;
use pass::*;

mod constants;
mod context;
mod pass;

pub struct VulkanContext {
    pub width: u32,
    pub height: u32,
    pub validation: bool,
    pub(crate) internal: Option<InternalContext>,
    pub(crate) pass: Option<Pass>,
}

#[derive(Debug, Clone, Copy)]
pub enum VulkanError {
    WindowNotInitialized,
    ValidationNotPresent,
}

impl VulkanContext {
    pub fn new(width: u32, height: u32, validation: bool) -> Self {
        Self { width, height, validation, internal: None, pass: None }
    }
}

pub fn update_context(context: &mut VulkanContext, window: &Window) {
    if let Some(internal) = context.internal.as_mut() {
        if window.internal.destroyed {
            context.internal = None;
        } else if window.events.contains(&WindowEvent::Resize) {
            resize_swapchain(
                internal,
                context.pass.as_mut(),
                window.inner_size.x,
                window.inner_size.y,
            );
        }
    } else if window.internal.initialized {
        match create_context(window, context.validation) {
            Ok(internal) => context.internal = Some(internal),
            Err(e) => utils::error(format!("{:?}", e)),
        }
    }
}

pub fn update_pass(context: &mut VulkanContext) {
    if let Some(internal) = context.internal.as_mut() {
        if context.pass.is_none() {
            context.pass = Some(create_pass(
                internal,
                &PassConfiguration { clear_color: [0.3, 0.1, 0.2, 1.0] },
            ));
        }
    }
}

pub fn draw_frame(context: &mut VulkanContext) {
    if let (Some(internal), Some(pass)) = (context.internal.as_mut(), context.pass.as_mut()) {
        // START FRAME
        if internal.out_of_date {
            utils::error("Swapchain outdated");
            return;
        }

        let present_index = unsafe {
            internal
                .device
                .wait_for_fences(&[internal.reuse_fence], true, u64::MAX)
                .expect("Wait failed");
            internal.device.reset_fences(&[internal.reuse_fence]).expect("Reset failed");

            let (present_index, _) = internal
                .swapchain_loader
                .acquire_next_image(
                    internal.swapchain,
                    u64::MAX,
                    internal.presentation_complete_semaphore,
                    vk::Fence::null(),
                )
                .expect("Acquire image failed");

            internal
                .device
                .reset_command_buffer(
                    internal.command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("Command buffer reset failed");
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            internal
                .device
                .begin_command_buffer(internal.command_buffer, &begin_info)
                .expect("Command buffer begin failed");

            present_index
        };

        begin_pass(internal, pass, present_index);
        end_pass(internal);

        // FINISH FRAME
        unsafe {
            internal
                .device
                .end_command_buffer(internal.command_buffer)
                .expect("End command buffer failed");

            let command_buffers = [internal.command_buffer];
            let wait_semaphores = [internal.presentation_complete_semaphore];
            let signal_semaphores = [internal.rendering_complete_semaphore];
            let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers)
                .wait_dst_stage_mask(&wait_dst_stage_mask)
                .wait_semaphores(&wait_semaphores)
                .signal_semaphores(&signal_semaphores);
            internal
                .device
                .queue_submit(internal.present_queue, &[submit_info], internal.reuse_fence)
                .expect("Submit failed");

            let wait_semaphores = [internal.rendering_complete_semaphore];
            let swapchains = [internal.swapchain];
            let image_indices = [present_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&wait_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);
            match internal.swapchain_loader.queue_present(internal.present_queue, &present_info) {
                Ok(_) => {}
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => internal.out_of_date = true,
                Err(e) => panic!("{e}"),
            }
        };
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        match (self.internal.take(), self.pass.take()) {
            (Some(mut internal), Some(mut pass)) => unsafe {
                internal.device.device_wait_idle().expect("Wait idle error");
                destroy_pass(&mut internal, &mut pass);
                destroy_context(&mut internal);
            },
            (Some(mut internal), None) => unsafe {
                internal.device.device_wait_idle().expect("Wait idle error");
                destroy_context(&mut internal);
            },
            (_, _) => {}
        }
    }
}
