use std::{
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use eframe::glow::{self, HasContext, NativeProgram};

use crate::{
    file_utils::{file_modified_time, Ms},
    shader::Shader,
};

struct ShaderData {
    vert_path: PathBuf,
    frag_path: PathBuf,
    native: Arc<Mutex<NativeProgram>>,
    vert_modified_time: Ms,
    frag_modified_time: Ms,
}
impl ShaderData {
    pub fn vert_shader_modified(&mut self) -> bool {
        let new_modified_time = match file_modified_time(&self.vert_path) {
            Some(time) => time,
            None => return false,
        };
        let old_modified_time = self.vert_modified_time;
        self.vert_modified_time = new_modified_time;
        old_modified_time != new_modified_time
    }
    pub fn frag_shader_modified(&mut self) -> bool {
        let new_modified_time = match file_modified_time(&self.frag_path) {
            Some(time) => time,
            None => return false,
        };
        let old_modified_time = self.frag_modified_time;
        self.frag_modified_time = new_modified_time;
        old_modified_time != new_modified_time
    }
}

pub struct ShaderReloader {
    shaders: Vec<ShaderData>,
}

impl ShaderReloader {
    pub fn new() -> Self {
        Self { shaders: vec![] }
    }
    pub fn add_shader(&mut self, shader: &Shader) {
        self.shaders.push(ShaderData {
            vert_path: shader.vert_path().clone(),
            frag_path: shader.frag_path().clone(),
            native: shader.native_mutex(),
            frag_modified_time: Ms(0),
            vert_modified_time: Ms(0),
        })
    }
    pub fn try_reload(&mut self, gl: &eframe::glow::Context) {
        for shader in self.shaders.iter_mut() {
            if shader.vert_shader_modified() || shader.frag_shader_modified() {
                let new_shader = match Shader::from_path(gl, &shader.frag_path, &shader.vert_path) {
                    Ok(new_shader) => new_shader,
                    Err(err) => {
                        eprintln!("Could not reload shader: {}", err);
                        continue;
                    },
                };
                // else {
                //     eprintln!("Error reloadingshad")
                //     continue;
                // };
                println!("Shader reloaded successfuly");
                let mut native = shader.native.lock().unwrap();
                unsafe { gl.delete_program(*native) };
                *native = new_shader.native();
            }
        }
    }
}
