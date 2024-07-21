use std::ffi::c_char;
use std::ffi::CString;

use ash::khr;
use ash::khr::surface;
use ash::khr::swapchain;
use ash::vk;

use crate::window::Window;

pub struct VulkanContext {
    entry: ash::Entry,
    instance: ash::Instance,

    surface: vk::SurfaceKHR,
    surface_loader: surface::Instance,

    device: ash::Device,
    physical_device: vk::PhysicalDevice,
}

impl VulkanContext {
    pub fn new(window: &Window) -> Self {
        let entry = unsafe { ash::Entry::load().expect("Vulkan not supported") };
        let instance = unsafe {
            let engine_name = CString::new("Best Engine").unwrap();
            let app_info = vk::ApplicationInfo::default()
                .api_version(vk::make_api_version(0, 1, 1, 0))
                .engine_name(&engine_name)
                .engine_version(1)
                .application_version(1);
            let extensions = get_required_extensions();
            //            let layers = get_layers();

            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(extensions)
                //                .enabled_layer_names(layers)
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

        println!("Picked device: {:?}", properties.device_name_as_c_str());

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

        let swapchain_loader = swapchain::Device::new(&instance, &device);
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(create_info, None)
                .expect("Swapchain create error")
        };

        Self {
            entry,
            instance,
            surface,
            surface_loader,
            device,
            physical_device,
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
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

fn get_required_extensions() -> &'static [*const c_char] {
    const EXTESIONS: [*const c_char; 2] = [
        khr::surface::NAME.as_ptr(),
        khr::win32_surface::NAME.as_ptr(),
    ];
    &EXTESIONS
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

//fn get_layers() -> &'static [*const c_char] {
//}
