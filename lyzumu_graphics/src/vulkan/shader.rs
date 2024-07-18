/*! Handles compilation of raw shader source code files.
 */

use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
    sync::Arc,
};

use anyhow::{Context, Result};
use ash::vk;

use super::{device::Device, DeviceShared};

const GLSL_VERSION_DIRECTIVE: &str = "#version 460 core";
const SHADER_INCLUDE_PRAGMA: &str = "#pragma INCLUDE";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Geometry,
    Compute,
    Mesh,
    Task,
}

impl ShaderStage {
    fn to_glslang_compiler_extension(self) -> String {
        match self {
            Self::Vertex => String::from("vert"),
            Self::Fragment => String::from("frag"),
            Self::Geometry => String::from("geom"),
            Self::Compute => String::from("comp"),
            Self::Mesh => String::from("mesh"),
            Self::Task => String::from("task"),
        }
    }

    fn to_glslang_stage_defines(self) -> String {
        match self {
            Self::Vertex => String::from("VERTEX"),
            Self::Fragment => String::from("FRAGMENT"),
            Self::Geometry => String::from("GEOMETRY"),
            Self::Compute => String::from("COMPUTE"),
            Self::Mesh => String::from("MESH"),
            Self::Task => String::from("TASK"),
        }
    }

    pub(crate) fn to_vulkan_shader_stage_flag(self) -> vk::ShaderStageFlags {
        use vk::ShaderStageFlags;

        match self {
            Self::Vertex => ShaderStageFlags::VERTEX,
            Self::Fragment => ShaderStageFlags::FRAGMENT,
            Self::Geometry => ShaderStageFlags::GEOMETRY,
            Self::Compute => ShaderStageFlags::COMPUTE,
            Self::Mesh => ShaderStageFlags::MESH_NV,
            Self::Task => ShaderStageFlags::TASK_NV,
        }
    }
}

fn read_shader_binary_file(file_name: &str) -> Result<Vec<u8>> {
    let bytes = fs::read(file_name)
        .with_context(|| format!("Failed to read shader binary file - {}", file_name))?;
    Ok(bytes)
}

fn read_shader_source_file(file_name: &str) -> Result<String> {
    let source_string = fs::read_to_string(file_name)
        .with_context(|| format!("Failed to read shader source file - {}", file_name))?;
    Ok(source_string)
}

/// Replaces includes in shader source string with the actual file include source code.
fn process_includes(content: &str, base_path: &str) -> Result<String> {
    let mut result_string = String::new();

    for line in content.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.starts_with(SHADER_INCLUDE_PRAGMA) {
            let start_index = trimmed_line.find('(').unwrap_or(trimmed_line.len());
            let end_index = trimmed_line.rfind(')').unwrap_or(start_index);

            let include_path = &trimmed_line[start_index + 1..end_index];
            let include_path_full = format!("{}/{}", base_path, include_path);

            let include_content = fs::read_to_string(&include_path_full).with_context(|| {
                format!("Failed to read shader include file - {}", include_path_full)
            })?;

            result_string.push_str(&process_includes(include_content.as_str(), base_path)?);
        } else if trimmed_line == GLSL_VERSION_DIRECTIVE {
            continue;
        } else {
            result_string.push_str(line);
            result_string.push('\n');
        }
    }

    Ok(result_string)
}

fn read_shader_source_file_with_includes(file_name: &str) -> Result<String> {
    let input_base_path = Path::new(file_name)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_str()
        .unwrap();

    let initial_shader_source = read_shader_source_file(file_name)?;

    let mut final_shader_source = String::from(GLSL_VERSION_DIRECTIVE);
    final_shader_source.push_str(&process_includes(
        initial_shader_source.as_str(),
        input_base_path,
    )?);

    Ok(final_shader_source)
}

fn compile_shader_through_glslangvalidator_cli(
    source_file_name: &str,
    destination_binary_file_name: &str,
    shader_stage: ShaderStage,
) -> Result<Vec<u8>> {
    let shader_source = read_shader_source_file_with_includes(source_file_name)?;

    let temp_file_name = "temp_shader";
    {
        let mut temp_file = File::create(temp_file_name)?;
        temp_file.write_all(shader_source.as_bytes())?;
    }

    let command_name = match std::env::consts::OS {
        "windows" => "glslangvalidator.exe",
        _ => "glslangValidator",
    };

    let command_output = Command::new(command_name)
        .arg(temp_file_name)
        .arg("-V")
        // XXX FIXME: Using 1.3 restricts the exeuction mode to LocalSizeID, this is presumably a bug(?). Use
        // vulkan1.2 for now.
        .args(["--target-env", "vulkan1.2"])
        .args(["-o", destination_binary_file_name])
        .args(["-S", shader_stage.to_glslang_compiler_extension().as_str()])
        .args(["--D", shader_stage.to_glslang_stage_defines().as_str()])
        .output()?;

    fs::remove_file(temp_file_name).with_context(|| "Failed to remove temp shader source file.")?;

    if command_output.status.success() {
        let shader_data = read_shader_binary_file(destination_binary_file_name)?;
        Ok(shader_data)
    } else {
        log::error!(
            "glslangValidator returned error: {:?}",
            String::from_utf8(command_output.stdout)
        );

        Err(anyhow::anyhow!(
            "Failed to compile shader through glslangvalidator!"
        ))
    }
}

pub struct ShaderModuleDescriptor<'a> {
    pub source_file_name: &'a str,
    pub shader_stage: ShaderStage,
}

impl<'a> ShaderModuleDescriptor<'a> {
    pub fn new(source_file_name: &'a str, shader_stage: ShaderStage) -> Self {
        Self {
            source_file_name,
            shader_stage,
        }
    }
}

pub struct ShaderModule {
    pub(crate) raw: vk::ShaderModule,
    pub stage: ShaderStage,
    device: Arc<DeviceShared>,
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_shader_module(self.raw, None);
        }
    }
}

impl Device {
    pub fn create_shader_module(&self, desc: ShaderModuleDescriptor) -> Result<ShaderModule> {
        let bytes = compile_shader_through_glslangvalidator_cli(
            desc.source_file_name,
            &(String::from(desc.source_file_name) + ".spv"),
            desc.shader_stage,
        )?;
        let mut cursor = std::io::Cursor::new(bytes);
        let code = ash::util::read_spv(&mut cursor)?;

        let create_info = vk::ShaderModuleCreateInfo::default().code(&code);
        let raw = unsafe { self.shared.raw.create_shader_module(&create_info, None)? };

        Ok(ShaderModule {
            raw,
            stage: desc.shader_stage,
            device: self.shared.clone(),
        })
    }
}
