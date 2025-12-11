use glow::*;
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

        unsafe {
            let program = gl.create_program().map_err(|e| format!("Cannot create program: {}", e))?;

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
        unsafe {
            self.gl.use_program(Some(self.program));
        }
    }

    pub fn unbind(&self) {
        unsafe {
            self.gl.use_program(None);
        }
    }
    
    pub fn get_uniform_location(&self, name: &str) -> Option<NativeUniformLocation> {
        unsafe {
            self.gl.get_uniform_location(self.program, name)
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
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

        unsafe {
            let program = gl.create_program().map_err(|e| format!("Cannot create program: {}", e))?;

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
        unsafe {
            self.gl.use_program(Some(self.program));
        }
    }

    pub fn unbind(&self) {
        unsafe {
            self.gl.use_program(None);
        }
    }
    
    pub fn get_uniform_location(&self, name: &str) -> Option<NativeUniformLocation> {
        unsafe {
            self.gl.get_uniform_location(self.program, name)
        }
    }
}

impl Drop for StockRemovalShaderProgram {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.program);
        }
    }
}

unsafe fn compile_shader(gl: &Context, source: &str, shader_type: u32) -> Result<NativeShader, String> {
    let shader = gl.create_shader(shader_type).map_err(|e| format!("Cannot create shader: {}", e))?;
    gl.shader_source(shader, source);
    gl.compile_shader(shader);

    if !gl.get_shader_compile_status(shader) {
        let log = gl.get_shader_info_log(shader);
        gl.delete_shader(shader);
        return Err(log);
    }

    Ok(shader)
}
