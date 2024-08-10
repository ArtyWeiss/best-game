use std::ffi::{c_char, CString};

use ash::{ext::debug_utils, khr::*, vk};

use crate::{utils, vulkan::constants::VALIDATION_NAME, vulkan::VulkanError};

pub unsafe fn create_instance(
    validation: bool,
) -> Result<(ash::Entry, ash::Instance), VulkanError> {
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

        entry.create_instance(&create_info, None).expect("Instance create error")
    };

    Ok((entry, instance))
}

fn get_layers(validation: bool) -> Vec<*const c_char> {
    if validation {
        vec![VALIDATION_NAME.as_ptr()]
    } else {
        vec![]
    }
}

fn get_required_extensions(validation: bool) -> Vec<*const c_char> {
    let mut extensions = vec![surface::NAME.as_ptr(), win32_surface::NAME.as_ptr()];
    if validation {
        extensions.push(debug_utils::NAME.as_ptr())
    }

    extensions
}

fn get_validation_support(entry: &ash::Entry) -> bool {
    let layer_properties = unsafe {
        entry.enumerate_instance_layer_properties().expect("Enumerate layer properties error")
    };
    layer_properties.iter().any(|l| {
        if let Ok(name) = l.layer_name_as_c_str() {
            name == VALIDATION_NAME
        } else {
            false
        }
    })
}
