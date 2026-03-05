//! # GLSL Shader Sources
//!
//! Contains GLSL vertex and fragment shader source code used by
//! the 3D renderer for toolpath visualization and grid rendering.

use glow::{
    Context, HasContext, NativeProgram, NativeShader, NativeUniformLocation, FRAGMENT_SHADER,
    VERTEX_SHADER,
};
use std::rc::Rc;

pub struct ShaderProgram {
    pub program: NativeProgram,
    pub gl: Rc<Context>,
}

impl ShaderProgram {
    pub fn new(gl: Rc<Context>) -> Result<Self, String> {
        let vertex_shader_source = r#"#version 330 core
            layout (location = 0) in vec3 aPos;
            layout (location = 1) in vec4 aColor;

            uniform mat4 uModelViewProjection;

            out vec4 vColor;

            void main() {
                gl_Position = uModelViewProjection * vec4(aPos, 1.0);
                vColor = aColor;
            }
        "#;

        let fragment_shader_source = r#"#version 330 core
            in vec4 vColor;
            out vec4 FragColor;

            void main() {
                FragColor = vColor;
            }
        "#;

        // SAFETY: All glow GL calls require unsafe. The GL context is valid
        // because it was created from a valid loader in the GLArea render callback.
        // Shaders are compiled, attached, linked, then detached and deleted.
        unsafe {
            let program = gl
                .create_program()
                .map_err(|e| format!("Cannot create program: {}", e))?;

            let vertex_shader = compile_shader(&gl, vertex_shader_source, VERTEX_SHADER)?;
            let fragment_shader = compile_shader(&gl, fragment_shader_source, FRAGMENT_SHADER)?;

            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }

            gl.detach_shader(program, vertex_shader);
            gl.detach_shader(program, fragment_shader);
            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);

            Ok(Self { program, gl })
        }
    }

    pub fn bind(&self) {
        // SAFETY: GL context is valid; program was successfully linked in new().
        unsafe {
            self.gl.use_program(Some(self.program));
        }
    }

    pub fn unbind(&self) {
        // SAFETY: GL context is valid; unbinding the current program is always safe.
        unsafe {
            self.gl.use_program(None);
        }
    }

    pub fn get_uniform_location(&self, name: &str) -> Option<NativeUniformLocation> {
        // SAFETY: GL context is valid; querying uniform locations is a read-only GL operation.
        unsafe { self.gl.get_uniform_location(self.program, name) }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        // SAFETY: GL context is valid; program handle is owned by this struct
        // and will not be used after deletion.
        unsafe {
            self.gl.delete_program(self.program);
        }
    }
}

pub struct StockRemovalShaderProgram {
    pub program: NativeProgram,
    pub gl: Rc<Context>,
}

impl StockRemovalShaderProgram {
    pub fn new(gl: Rc<Context>) -> Result<Self, String> {
        let vertex_shader_source = r#"#version 330 core
            layout (location = 0) in vec3 aPos;
            layout (location = 1) in vec3 aNormal;
            layout (location = 2) in vec4 aColor;

            uniform mat4 uModelViewProjection;
            uniform mat3 uNormalMatrix;

            out vec3 vNormal;
            out vec4 vColor;

            void main() {
                gl_Position = uModelViewProjection * vec4(aPos, 1.0);
                vNormal = normalize(uNormalMatrix * aNormal);
                vColor = aColor;
            }
        "#;

        let fragment_shader_source = r#"#version 330 core
            in vec3 vNormal;
            in vec4 vColor;

            uniform vec3 uLightDir;

            out vec4 FragColor;

            void main() {
                vec3 normal = normalize(vNormal);
                vec3 lightDir = normalize(uLightDir);
                float diffuse = max(dot(normal, lightDir), 0.1);

                // Simple specular
                vec3 viewDir = vec3(0.0, 0.0, 1.0);
                vec3 halfDir = normalize(lightDir + viewDir);
                float specular = pow(max(dot(normal, halfDir), 0.0), 24.0) * 0.3;

                vec3 color = vColor.rgb * diffuse + specular;
                FragColor = vec4(color, vColor.a);
            }
        "#;

        // SAFETY: All glow GL calls require unsafe. The GL context is valid
        // because it was created from a valid loader in the GLArea render callback.
        // Shaders are compiled, attached, linked, then detached and deleted.
        unsafe {
            let program = gl
                .create_program()
                .map_err(|e| format!("Cannot create program: {}", e))?;

            let vertex_shader = compile_shader(&gl, vertex_shader_source, VERTEX_SHADER)?;
            let fragment_shader = compile_shader(&gl, fragment_shader_source, FRAGMENT_SHADER)?;

            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }

            gl.detach_shader(program, vertex_shader);
            gl.detach_shader(program, fragment_shader);
            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);

            Ok(Self { program, gl })
        }
    }

    pub fn bind(&self) {
        // SAFETY: GL context is valid; program was successfully linked in new().
        unsafe {
            self.gl.use_program(Some(self.program));
        }
    }

    pub fn unbind(&self) {
        // SAFETY: GL context is valid; unbinding the current program is always safe.
        unsafe {
            self.gl.use_program(None);
        }
    }

    pub fn get_uniform_location(&self, name: &str) -> Option<NativeUniformLocation> {
        // SAFETY: GL context is valid; querying uniform locations is a read-only GL operation.
        unsafe { self.gl.get_uniform_location(self.program, name) }
    }
}

impl Drop for StockRemovalShaderProgram {
    fn drop(&mut self) {
        // SAFETY: GL context is valid; program handle is owned by this struct
        // and will not be used after deletion.
        unsafe {
            self.gl.delete_program(self.program);
        }
    }
}

// SAFETY: This function is unsafe because all glow GL calls require unsafe.
// Caller must ensure the GL context is current. The shader is created, compiled,
// and on failure cleaned up before returning an error.
unsafe fn compile_shader(
    gl: &Context,
    source: &str,
    shader_type: u32,
) -> Result<NativeShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .map_err(|e| format!("Cannot create shader: {}", e))?;
    gl.shader_source(shader, source);
    gl.compile_shader(shader);

    if !gl.get_shader_compile_status(shader) {
        let log = gl.get_shader_info_log(shader);
        gl.delete_shader(shader);
        return Err(log);
    }

    Ok(shader)
}
