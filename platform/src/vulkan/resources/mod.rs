use crate::utils;

use super::{InternalContext, VulkanError};
use ash::vk;

pub struct Resources {
    pub pipelines: Vec<vk::Pipeline>,
}

pub struct PipelineConfig<'a> {
    pub vertext_shader_source: &'a [u8],
    pub fragment_shader_source: &'a [u8],
}

impl Resources {
    pub fn create_pipeline(
        &mut self,
        context: &mut InternalContext,
        config: PipelineConfig,
    ) -> Result<u32, VulkanError> {
        utils::trace(format!(
            "Creating pipeline from VS {}b and FS {}b",
            config.vertext_shader_source.len(),
            config.fragment_shader_source.len()
        ));
        let id = 0;
        Ok(id)
    }
}

pub const fn create_resources() -> Resources {
    Resources { pipelines: vec![] }
}

pub fn destroy_resources(resources: &mut Resources, context: &InternalContext) {
    unsafe {
        for pip in resources.pipelines.iter() {
            context.device.destroy_pipeline(*pip, None);
        }
    }
}
