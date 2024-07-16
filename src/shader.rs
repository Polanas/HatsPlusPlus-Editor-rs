use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{bail, Result};
use bevy_math::{Vec2, Vec3};
use eframe::glow::{self, NativeUniformLocation};
use eframe::glow::{HasContext, NativeProgram};

#[derive(Clone, Debug)]
pub struct Shader {
    native: Arc<Mutex<NativeProgram>>,
    frag_path: PathBuf,
    vert_path: PathBuf,
    uniforms: HashMap<String, NativeUniformLocation>,
}

impl Shader {
    pub fn from_path(
        gl: &eframe::glow::Context,
        frag_path: impl AsRef<Path>,
        vert_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let vert = std::fs::read_to_string(vert_path.as_ref())?;
        let frag = std::fs::read_to_string(frag_path.as_ref())?;
        unsafe {
            let program = match gl.create_program() {
                Ok(program) => program,
                Err(_) => bail!("could not create shader program"),
            };
            let shaders = [(glow::FRAGMENT_SHADER, frag), (glow::VERTEX_SHADER, vert)]
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, shader_source);
                    gl.compile_shader(shader);
                    let shader_name = match *shader_type {
                        glow::FRAGMENT_SHADER => "fragment shader",
                        glow::VERTEX_SHADER => "vertex shader",
                        _ => "some other shader",
                    };
                    if !gl.get_shader_compile_status(shader) {
                        bail!(
                            "failed to compile {shader_name}: {}",
                            gl.get_shader_info_log(shader)
                        );
                    }
                    gl.attach_shader(program, shader);
                    Ok(shader)
                })
                .collect::<Result<Vec<_>>>()?;
            // let shaders: Result<Vec<> = shaders.into_iter().collect::<Vec<_>>();
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                bail!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }
            let uniforms_amount = gl.get_active_uniforms(program);
            let uniforms: HashMap<_, _> = (0..=uniforms_amount)
                .flat_map(|i| gl.get_active_uniform(program, i))
                .flat_map(|active| {
                    gl.get_uniform_location(program, &active.name)
                        .map(|l| (active.name, l))
                })
                .collect();
            Ok(Shader {
                native: Arc::new(Mutex::new(program)),
                frag_path: frag_path.as_ref().to_owned(),
                vert_path: vert_path.as_ref().to_owned(),
                uniforms,
            })
        }
    }
    pub fn from_text_with_path(
        gl: &eframe::glow::Context,
        frag_path: impl AsRef<Path>,
        frag: &str,
        vert_path: impl AsRef<Path>,
        vert: &str,
    ) -> Result<Self> {
        unsafe {
            let program = match gl.create_program() {
                Ok(program) => program,
                Err(_) => bail!("could not create shader program"),
            };
            let shaders = [(glow::FRAGMENT_SHADER, frag), (glow::VERTEX_SHADER, vert)]
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, shader_source);
                    gl.compile_shader(shader);
                    let shader_name = match *shader_type {
                        glow::FRAGMENT_SHADER => "fragment shader",
                        glow::VERTEX_SHADER => "vertex shader",
                        _ => "some other shader",
                    };
                    if !gl.get_shader_compile_status(shader) {
                        bail!(
                            "failed to compile {shader_name}: {}",
                            gl.get_shader_info_log(shader)
                        );
                    }
                    gl.attach_shader(program, shader);
                    Ok(shader)
                })
                .collect::<Result<Vec<_>>>()?;
            // let shaders: Result<Vec<> = shaders.into_iter().collect::<Vec<_>>();
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                bail!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }
            let uniforms_amount = gl.get_active_uniforms(program);
            let uniforms: HashMap<_, _> = (0..=uniforms_amount)
                .flat_map(|i| gl.get_active_uniform(program, i))
                .flat_map(|active| {
                    gl.get_uniform_location(program, &active.name)
                        .map(|l| (active.name, l))
                })
                .collect();
            Ok(Shader {
                native: Arc::new(Mutex::new(program)),
                frag_path: frag_path.as_ref().to_owned(),
                vert_path: vert_path.as_ref().to_owned(),
                uniforms,
            })
        }
    }

    pub fn uniforms(&self) -> &HashMap<String, NativeUniformLocation> {
        &self.uniforms
    }

    pub fn vert_path(&self) -> &PathBuf {
        &self.vert_path
    }

    pub fn frag_path(&self) -> &PathBuf {
        &self.frag_path
    }

    pub fn native(&self) -> NativeProgram {
        *self.native.lock().unwrap()
    }

    pub fn native_mutex(&self) -> Arc<Mutex<NativeProgram>> {
        self.native.clone()
    }

    pub fn activate(&self, gl: &eframe::glow::Context) {
        unsafe {
            gl.use_program(Some(self.native()));
        }
    }

    pub fn set_f32(&self, gl: &eframe::glow::Context, name: &str, value: f32) {
        unsafe { gl.uniform_1_f32(self.uniforms().get(name), value) };
    }

    pub fn set_i32(&self, gl: &eframe::glow::Context, name: &str, value: i32) {
        unsafe { gl.uniform_1_i32(self.uniforms().get(name), value) };
    }

    pub fn set_vec2(&self, gl: &eframe::glow::Context, name: &str, value: Vec2) {
        unsafe { gl.uniform_2_f32(self.uniforms().get(name), value.x, value.y) };
    }
    #[allow(dead_code)]
    pub fn set_vec3(&self, gl: &eframe::glow::Context, name: &str, value: Vec3) {
        unsafe { gl.uniform_3_f32(self.uniforms().get(name), value.x, value.y, value.z) };
    }
}
