use std::borrow::Cow;
use std::ffi::c_char;
use std::ffi::CStr;
use std::ffi::CString;

use ash::ext::debug_utils;
use ash::khr;
use ash::khr::surface;
use ash::khr::swapchain;
use ash::vk;
use ash::vk::Extent2D;

use crate::utils;
use crate::window::Window;
use crate::window::WindowEvent;

use constants::VALIDATION_NAME;
mod constants;

pub struct VulkanContext {
    pub width: u32,
    pub height: u32,
    pub validation: bool,
    pub(crate) internal: Option<InternalContext>,
    pub(crate) pass: Option<Pass>,
}

pub(crate) struct InternalContext {
    out_of_date: bool,

    entry: ash::Entry,
    instance: ash::Instance,

    surface: vk::SurfaceKHR,
    surface_loader: surface::Instance,
    surface_format: vk::SurfaceFormatKHR,
    surface_resolution: vk::Extent2D,
    present_mode: vk::PresentModeKHR,

    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    present_queue: vk::Queue,

    swapchain_loader: swapchain::Device,
    swapchain: vk::SwapchainKHR,

    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    reuse_fence: vk::Fence,

    swapchain_image_views: Vec<vk::ImageView>,
    rendering_complete_semaphore: vk::Semaphore,
    presentation_complete_semaphore: vk::Semaphore,

    debug_utils_loader: debug_utils::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

pub(crate) struct Pass {
    raw: vk::RenderPass,
    clear_value: vk::ClearValue,
    framebuffers: Vec<vk::Framebuffer>,
}

#[derive(Debug, Clone, Copy)]
pub enum VulkanError {
    WindowNotInitialized,
    ValidationNotPresent,
}

impl VulkanContext {
    pub fn new(width: u32, height: u32, validation: bool) -> Self {
        Self {
            width,
            height,
            validation,
            internal: None,
            pass: None,
        }
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
            context.pass = Some(create_pass(internal));
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
            internal
                .device
                .reset_fences(&[internal.reuse_fence])
                .expect("Reset failed");

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

        unsafe {
            let clear_values = [pass.clear_value];
            let begin_info = vk::RenderPassBeginInfo::default()
                .render_pass(pass.raw)
                .clear_values(&clear_values)
                .render_area(internal.surface_resolution.into())
                .framebuffer(pass.framebuffers[present_index as usize]);
            internal.device.cmd_begin_render_pass(
                internal.command_buffer,
                &begin_info,
                vk::SubpassContents::INLINE,
            );
            internal.device.cmd_end_render_pass(internal.command_buffer);
        }

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
            match internal
                .swapchain_loader
                .queue_present(internal.present_queue, &present_info)
            {
                Ok(_) => {}
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => internal.out_of_date = true,
                Err(e) => panic!("{e}"),
            }
        };

        //
        //        // ACTUAL RENDERING CODE HERE
        //
        //        unsafe {
        //            let clear_values = [pass.clear_value];
        //            let begin_info = vk::RenderPassBeginInfo::default()
        //                .render_pass(pass.raw)
        //                .clear_values(&clear_values)
        //                .render_area(internal.surface_resolution.into())
        //                .framebuffer(pass.framebuffers[present_index as usize]);
        //            internal.device.cmd_begin_render_pass(
        //                internal.command_buffer,
        //                &begin_info,
        //                vk::SubpassContents::INLINE,
        //            );
        //            internal.device.cmd_end_render_pass(internal.command_buffer);
        //        }
        //
        //        // ACTUAL RENDERING CODE HERE
        //
    }
}

fn create_context(window: &Window, validation: bool) -> Result<InternalContext, VulkanError> {
    if window.hwnd() == 0 || window.hinstance() == 0 {
        return Err(VulkanError::WindowNotInitialized);
    }
    let entry = unsafe { ash::Entry::load().expect("Vulkan not supported") };

    let validation_support = get_validation_support(&entry);
    if !validation_support && validation {
        utils::error("Validation requested but not present");
        return Err(VulkanError::ValidationNotPresent);
    }

    let instance = unsafe {
        let engine_name = CString::new("Best Engine").unwrap();
        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::make_api_version(0, 1, 1, 0))
            .engine_name(&engine_name)
            .engine_version(1)
            .application_version(1);
        let layers = get_layers(validation);
        let extensions = get_required_extensions(validation);

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers)
            .flags(vk::InstanceCreateFlags::default());

        entry
            .create_instance(&create_info, None)
            .expect("Instance create error")
    };

    let surface = create_surface(&entry, &instance, window);
    let surface_loader = surface::Instance::new(&entry, &instance);
    let (physical_device, queue_family_index) =
        unsafe { pick_physical_device(&instance, &surface_loader, surface) };
    let properties = unsafe { instance.get_physical_device_properties(physical_device) };

    utils::trace(format!(
        "Picked device: {:?}",
        properties.device_name_as_c_str()
    ));

    let priorities = [1.0];
    let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities)];

    let device_extensions = [khr::swapchain::NAME.as_ptr()];
    let device_features = vk::PhysicalDeviceFeatures::default();
    let create_info = vk::DeviceCreateInfo::default()
        .enabled_extension_names(&device_extensions)
        .enabled_features(&device_features)
        .queue_create_infos(&queue_create_infos);

    let device = unsafe {
        instance
            .create_device(physical_device, &create_info, None)
            .expect("Device create error")
    };
    let present_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

    let surface_format = unsafe {
        surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .expect("No formats")[0]
    };
    let surface_capabilities = unsafe {
        surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .expect("No caps")
    };
    let desired_image_count = 3u32;
    let image_extent = if surface_capabilities.current_extent.width == u32::MAX {
        Extent2D {
            width: window.inner_size.x,
            height: window.inner_size.y,
        }
    } else {
        surface_capabilities.current_extent
    };
    let pre_transform = if surface_capabilities
        .supported_transforms
        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
    {
        vk::SurfaceTransformFlagsKHR::IDENTITY
    } else {
        surface_capabilities.current_transform
    };
    let supported_present_modes = unsafe {
        surface_loader
            .get_physical_device_surface_present_modes(physical_device, surface)
            .expect("No present modes")
    };
    let present_mode = if supported_present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
        vk::PresentModeKHR::MAILBOX
    } else {
        vk::PresentModeKHR::FIFO
    };

    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(desired_image_count)
        .present_mode(present_mode)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_extent(image_extent)
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .pre_transform(pre_transform)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .clipped(true)
        .image_array_layers(1);

    let swapchain_loader = swapchain::Device::new(&instance, &device);
    let swapchain = unsafe {
        swapchain_loader
            .create_swapchain(&create_info, None)
            .expect("Swapchain create error")
    };

    let create_info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family_index);
    let command_pool = unsafe {
        device
            .create_command_pool(&create_info, None)
            .expect("Create command pool failed")
    };
    let allocate_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .command_buffer_count(1);
    let command_buffer = unsafe {
        device
            .allocate_command_buffers(&allocate_info)
            .expect("Cant allocate command buffer")[0]
    };

    let create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
    let reuse_fence = unsafe {
        device
            .create_fence(&create_info, None)
            .expect("Failed to create fence")
    };

    let present_images = unsafe {
        swapchain_loader
            .get_swapchain_images(swapchain)
            .expect("No images")
    };

    let swapchain_image_views =
        create_swapchain_image_views(&device, &present_images, surface_format.format);

    let create_info = vk::SemaphoreCreateInfo::default();
    let rendering_complete_semaphore = unsafe {
        device
            .create_semaphore(&create_info, None)
            .expect("Cant create semaphore")
    };
    let presentation_complete_semaphore = unsafe {
        device
            .create_semaphore(&create_info, None)
            .expect("Cant create semaphore")
    };

    let create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(vulkan_debug_callback));
    let debug_utils_loader = debug_utils::Instance::new(&entry, &instance);
    let debug_messenger = unsafe {
        debug_utils_loader
            .create_debug_utils_messenger(&create_info, None)
            .expect("Debug callback error")
    };

    Ok(InternalContext {
        out_of_date: false,
        entry,
        instance,
        surface,
        surface_loader,
        device,
        physical_device,
        present_queue,
        swapchain_loader,
        swapchain,
        command_pool,
        command_buffer,
        reuse_fence,
        swapchain_image_views,
        rendering_complete_semaphore,
        presentation_complete_semaphore,
        surface_format,
        surface_resolution: image_extent,
        present_mode,
        debug_utils_loader,
        debug_messenger,
    })
}

fn create_swapchain_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> Vec<vk::ImageView> {
    images
        .iter()
        .map(|&image| {
            let create_info = vk::ImageViewCreateInfo::default()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::R,
                    g: vk::ComponentSwizzle::G,
                    b: vk::ComponentSwizzle::B,
                    a: vk::ComponentSwizzle::A,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(image);
            unsafe {
                device
                    .create_image_view(&create_info, None)
                    .expect("Cant create image view")
            }
        })
        .collect()
}

fn create_framebuffers(
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
                device
                    .create_framebuffer(&create_info, None)
                    .expect("Cant create framebuffer")
            }
        })
        .collect()
}

fn resize_swapchain(
    context: &mut InternalContext,
    mut pass: Option<&mut Pass>,
    width: u32,
    height: u32,
) {
    unsafe {
        context.device.device_wait_idle().expect("Wait idle error");
    }

    // CLEANUP
    if let Some(pass) = pass.as_mut() {
        for fb in pass.framebuffers.drain(..) {
            unsafe { context.device.destroy_framebuffer(fb, None) };
        }
    }

    let desired_image_count = context.swapchain_image_views.len() as u32;
    for image_view in context.swapchain_image_views.drain(..) {
        unsafe {
            context.device.destroy_image_view(image_view, None);
        }
    }
    unsafe {
        context
            .swapchain_loader
            .destroy_swapchain(context.swapchain, None);
    }

    // CREATION
    // TODO: Кидается ошибкой, если не вызывать get_surface_capabilites()
    // Почему??
    let surface_capabilities = unsafe {
        context
            .surface_loader
            .get_physical_device_surface_capabilities(context.physical_device, context.surface)
            .expect("Failed to get surface capabilities")
    };
    utils::trace(format!(
        "Surface size {}x{}",
        surface_capabilities.min_image_extent.width, surface_capabilities.min_image_extent.height
    ));

    context.surface_resolution = vk::Extent2D { width, height };
    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(context.surface)
        .min_image_count(desired_image_count)
        .present_mode(context.present_mode)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_extent(context.surface_resolution)
        .image_color_space(context.surface_format.color_space)
        .image_format(context.surface_format.format)
        .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .clipped(true)
        .image_array_layers(1);
    context.swapchain = unsafe {
        context
            .swapchain_loader
            .create_swapchain(&create_info, None)
            .expect("Swapchain create error")
    };
    let present_images = unsafe {
        context
            .swapchain_loader
            .get_swapchain_images(context.swapchain)
            .expect("Failed to get swapchain images")
    };
    context.swapchain_image_views = create_swapchain_image_views(
        &context.device,
        &present_images,
        context.surface_format.format,
    );

    if let Some(pass) = pass.as_mut() {
        pass.framebuffers = create_framebuffers(
            &context.device,
            &context.swapchain_image_views,
            pass.raw,
            context.surface_resolution.width,
            context.surface_resolution.height,
        );
    }

    utils::trace(format!(
        "Resized to {}x{}",
        context.surface_resolution.width, context.surface_resolution.height
    ));
}

unsafe fn destroy_context(context: &mut InternalContext) {
    context.device.device_wait_idle().expect("Wait idle error");
    context
        .device
        .destroy_semaphore(context.rendering_complete_semaphore, None);
    context
        .device
        .destroy_semaphore(context.presentation_complete_semaphore, None);
    context.device.destroy_fence(context.reuse_fence, None);
    for image_view in context.swapchain_image_views.iter() {
        context.device.destroy_image_view(*image_view, None);
    }
    context
        .device
        .destroy_command_pool(context.command_pool, None);

    context
        .debug_utils_loader
        .destroy_debug_utils_messenger(context.debug_messenger, None);

    context
        .swapchain_loader
        .destroy_swapchain(context.swapchain, None);
    context.device.destroy_device(None);
    context
        .surface_loader
        .destroy_surface(context.surface, None);
    context.instance.destroy_instance(None);
}

unsafe fn destroy_pass(context: &InternalContext, pass: &mut Pass) {
    for fb in pass.framebuffers.iter() {
        context.device.destroy_framebuffer(*fb, None);
    }
    context.device.destroy_render_pass(pass.raw, None);
}

fn get_validation_support(entry: &ash::Entry) -> bool {
    let layer_properties = unsafe {
        entry
            .enumerate_instance_layer_properties()
            .expect("Enumerate layer properties error")
    };
    layer_properties.iter().any(|l| {
        if let Ok(name) = l.layer_name_as_c_str() {
            name == VALIDATION_NAME
        } else {
            false
        }
    })
}

#[allow(unused)]
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            utils::error(format!("{message_id_name} : {message}"))
        }
        _ => utils::trace(format!("{message_id_name} : {message}")),
    }

    vk::FALSE
}

fn create_pass(internal: &mut InternalContext) -> Pass {
    let attachments = [vk::AttachmentDescription {
        format: internal.surface_format.format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::DONT_CARE,
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
        internal
            .device
            .create_render_pass(&create_info, None)
            .expect("Cant create render pass")
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
            color: vk::ClearColorValue { float32: [0.0, 0.0, 1.0, 1.0] },
        },
        framebuffers,
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

unsafe fn pick_physical_device(
    instance: &ash::Instance,
    surface_loader: &surface::Instance,
    surface: vk::SurfaceKHR,
) -> (vk::PhysicalDevice, u32) {
    let all_devices = instance
        .enumerate_physical_devices()
        .expect("Enumerate physical devices error");
    all_devices
        .iter()
        .find_map(|device| {
            instance
                .get_physical_device_queue_family_properties(*device)
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    let suppors_graphics = info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                    let supports_surface = surface_loader
                        .get_physical_device_surface_support(*device, index as u32, surface)
                        .expect("Get physical device surface support error");
                    if suppors_graphics && supports_surface {
                        Some((*device, index as u32))
                    } else {
                        None
                    }
                })
        })
        .expect("No suitable physical device")
}

fn get_layers(validation: bool) -> Vec<*const c_char> {
    if validation {
        vec![VALIDATION_NAME.as_ptr()]
    } else {
        vec![]
    }
}

fn get_required_extensions(validation: bool) -> Vec<*const c_char> {
    let mut extensions = vec![
        khr::surface::NAME.as_ptr(),
        khr::win32_surface::NAME.as_ptr(),
    ];
    if validation {
        extensions.push(debug_utils::NAME.as_ptr())
    }

    extensions
}

fn create_surface(entry: &ash::Entry, instance: &ash::Instance, window: &Window) -> vk::SurfaceKHR {
    let create_info = vk::Win32SurfaceCreateInfoKHR::default()
        .hwnd(window.hwnd())
        .hinstance(window.hinstance());
    let surface_fn = khr::win32_surface::Instance::new(entry, instance);
    unsafe {
        surface_fn
            .create_win32_surface(&create_info, None)
            .expect("Surface create error")
    }
}
