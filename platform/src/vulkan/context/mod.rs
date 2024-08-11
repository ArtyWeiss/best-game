use ash::{ext::debug_utils, khr::*, vk};
use std::{borrow::Cow, ffi::CStr};

use super::{constants::FRAMES_IN_FLIGHT, create_framebuffers, Pass, VulkanError};
use crate::{utils, window::Window};

use instance::*;
pub mod instance;

pub struct InternalContext {
    pub out_of_date: bool,

    _entry: ash::Entry,
    pub instance: ash::Instance,

    pub surface: vk::SurfaceKHR,
    pub surface_loader: surface::Instance,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,
    pub present_mode: vk::PresentModeKHR,

    pub device: ash::Device,
    pub physical_device: vk::PhysicalDevice,
    pub present_queue: vk::Queue,
    pub command_pool: vk::CommandPool,

    pub swapchain_loader: swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_image_views: Vec<vk::ImageView>,

    pub current_frame: usize,
    pub frames: [Frame; FRAMES_IN_FLIGHT],
    pub present_index: Option<u32>,

    pub debug_utils_loader: debug_utils::Instance,
    pub debug_messenger: vk::DebugUtilsMessengerEXT,
}

pub struct Frame {
    pub command_buffer: vk::CommandBuffer,
    pub reuse_fence: vk::Fence,
    pub rendering_complete_semaphore: vk::Semaphore,
    pub presentation_complete_semaphore: vk::Semaphore,
}

pub fn create_context(window: &Window, validation: bool) -> Result<InternalContext, VulkanError> {
    if window.hwnd() == 0 || window.hinstance() == 0 {
        return Err(VulkanError::WindowNotInitialized);
    }
    let (entry, instance) = unsafe { create_instance(validation)? };

    let surface = create_surface(&entry, &instance, window);
    let surface_loader = surface::Instance::new(&entry, &instance);
    let (physical_device, queue_family_index) =
        unsafe { pick_physical_device(&instance, &surface_loader, surface) };
    let properties = unsafe { instance.get_physical_device_properties(physical_device) };

    utils::trace(format!(
        "Picked device: {:?}",
        properties.device_name_as_c_str().unwrap_or_default()
    ));

    let priorities = [1.0];
    let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities)];

    let device_extensions = [swapchain::NAME.as_ptr()];
    let device_features = vk::PhysicalDeviceFeatures::default();
    let create_info = vk::DeviceCreateInfo::default()
        .enabled_extension_names(&device_extensions)
        .enabled_features(&device_features)
        .queue_create_infos(&queue_create_infos);

    let device = unsafe {
        instance.create_device(physical_device, &create_info, None).expect("Device create error")
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
        vk::Extent2D { width: window.inner_size.x, height: window.inner_size.y }
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
        swapchain_loader.create_swapchain(&create_info, None).expect("Swapchain create error")
    };

    let create_info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family_index);
    let command_pool = unsafe {
        device.create_command_pool(&create_info, None).expect("Create command pool failed")
    };

    let present_images = unsafe {
        swapchain_loader.get_swapchain_images(swapchain).expect("Failed to get swapchain images")
    };
    let swapchain_image_views =
        create_swapchain_image_views(&device, &present_images, surface_format.format);

    let command_buffer_allocate_info =
        vk::CommandBufferAllocateInfo::default().command_pool(command_pool).command_buffer_count(1);
    let fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
    let semaphore_create_info = vk::SemaphoreCreateInfo::default();

    let frames = std::array::from_fn(|_| {
        let command_buffer = unsafe {
            device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .expect("Cant allocate command buffer")[0]
        };
        let reuse_fence = unsafe {
            device.create_fence(&fence_create_info, None).expect("Failed to create fence")
        };
        let rendering_complete_semaphore = unsafe {
            device.create_semaphore(&semaphore_create_info, None).expect("Cant create semaphore")
        };
        let presentation_complete_semaphore = unsafe {
            device.create_semaphore(&semaphore_create_info, None).expect("Cant create semaphore")
        };
        Frame {
            command_buffer,
            reuse_fence,
            rendering_complete_semaphore,
            presentation_complete_semaphore,
        }
    });

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
        _entry: entry,
        instance,
        surface,
        surface_loader,
        device,
        physical_device,
        present_queue,
        swapchain_loader,
        swapchain,
        command_pool,
        swapchain_image_views,
        surface_format,
        surface_resolution: image_extent,
        present_mode,
        debug_utils_loader,
        debug_messenger,
        current_frame: 0,
        present_index: None,
        frames,
    })
}

pub unsafe fn destroy_context(context: &mut InternalContext) {
    context.device.device_wait_idle().expect("Wait idle error");
    for f in context.frames.iter() {
        context.device.destroy_semaphore(f.rendering_complete_semaphore, None);
        context.device.destroy_semaphore(f.presentation_complete_semaphore, None);
        context.device.destroy_fence(f.reuse_fence, None);
    }
    for image_view in context.swapchain_image_views.iter() {
        context.device.destroy_image_view(*image_view, None);
    }
    context.device.destroy_command_pool(context.command_pool, None);

    context.debug_utils_loader.destroy_debug_utils_messenger(context.debug_messenger, None);

    context.swapchain_loader.destroy_swapchain(context.swapchain, None);
    context.device.destroy_device(None);
    context.surface_loader.destroy_surface(context.surface, None);
    context.instance.destroy_instance(None);
}

pub fn resize_swapchain(
    context: &mut InternalContext,
    mut pass: Option<&mut Pass>,
    width: u32,
    height: u32,
) {
    if width == 0 || height == 0 {
        return;
    }
    // TODO: Кидается ошибкой, если не вызывать get_surface_capabilites()
    // Почему??
    let surface_capabilities = unsafe {
        context
            .surface_loader
            .get_physical_device_surface_capabilities(context.physical_device, context.surface)
            .expect("Failed to get surface capabilities")
    };
    if width < surface_capabilities.min_image_extent.width
        || height < surface_capabilities.min_image_extent.height
        || width > surface_capabilities.max_image_extent.width
        || height > surface_capabilities.max_image_extent.height
    {
        utils::error("Expected size is outside of bounds, provided by surface");
        return;
    }

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
        context.swapchain_loader.destroy_swapchain(context.swapchain, None);
    }

    // CREATION
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
    context.out_of_date = false;

    utils::trace(format!(
        "Swapchain resized: {}x{}",
        context.surface_resolution.width, context.surface_resolution.height
    ));
}

unsafe fn pick_physical_device(
    instance: &ash::Instance,
    surface_loader: &surface::Instance,
    surface: vk::SurfaceKHR,
) -> (vk::PhysicalDevice, u32) {
    let all_devices =
        instance.enumerate_physical_devices().expect("Enumerate physical devices error");
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

fn create_surface(entry: &ash::Entry, instance: &ash::Instance, window: &Window) -> vk::SurfaceKHR {
    let create_info =
        vk::Win32SurfaceCreateInfoKHR::default().hwnd(window.hwnd()).hinstance(window.hinstance());
    let surface_fn = win32_surface::Instance::new(entry, instance);
    unsafe { surface_fn.create_win32_surface(&create_info, None).expect("Surface create error") }
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
            unsafe { device.create_image_view(&create_info, None).expect("Cant create image view") }
        })
        .collect()
}
