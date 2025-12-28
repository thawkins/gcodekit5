//! # OpenGL Shaders for 3D Mesh Rendering
//!
//! Vertex and fragment shaders for rendering 3D meshes with proper lighting.

pub const MESH_VERTEX_SHADER: &str = r#"
#version 330 core

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec4 color;

uniform mat4 mvp_matrix;
uniform mat4 model_matrix;
uniform mat4 view_matrix;
uniform mat4 projection_matrix;
uniform mat3 normal_matrix;

// Lighting uniforms
uniform vec3 light_direction;
uniform vec3 light_color;
uniform vec3 ambient_color;
uniform vec3 camera_position;

// Material uniforms
uniform vec4 material_diffuse;
uniform vec4 material_ambient;
uniform vec4 material_specular;
uniform float material_shininess;
uniform float material_alpha;

out vec3 frag_position;
out vec3 frag_normal;
out vec4 frag_color;
out vec3 frag_light_direction;
out vec3 frag_view_direction;

void main() {
    // Transform position
    vec4 world_position = model_matrix * vec4(position, 1.0);
    gl_Position = mvp_matrix * vec4(position, 1.0);
    
    // Pass world position for lighting calculations
    frag_position = world_position.xyz;
    
    // Transform normal to world space
    frag_normal = normalize(normal_matrix * normal);
    
    // Use vertex color or material color
    frag_color = color * material_diffuse;
    
    // Light and view directions in world space
    frag_light_direction = normalize(-light_direction);
    frag_view_direction = normalize(camera_position - frag_position);
}
"#;

pub const MESH_FRAGMENT_SHADER: &str = r#"
#version 330 core

in vec3 frag_position;
in vec3 frag_normal;
in vec4 frag_color;
in vec3 frag_light_direction;
in vec3 frag_view_direction;

// Lighting uniforms
uniform vec3 light_color;
uniform vec3 ambient_color;

// Material uniforms
uniform vec4 material_diffuse;
uniform vec4 material_ambient;
uniform vec4 material_specular;
uniform float material_shininess;
uniform float material_alpha;
uniform bool wireframe_mode;

out vec4 FragColor;

void main() {
    if (wireframe_mode) {
        // Simple wireframe rendering
        FragColor = vec4(frag_color.rgb, material_alpha);
        return;
    }
    
    vec3 normal = normalize(frag_normal);
    vec3 light_dir = normalize(frag_light_direction);
    vec3 view_dir = normalize(frag_view_direction);
    
    // Ambient lighting
    vec3 ambient = ambient_color * material_ambient.rgb;
    
    // Diffuse lighting
    float diff = max(dot(normal, light_dir), 0.0);
    vec3 diffuse = diff * light_color * frag_color.rgb;
    
    // Specular lighting (Blinn-Phong)
    vec3 halfway_dir = normalize(light_dir + view_dir);
    float spec = pow(max(dot(normal, halfway_dir), 0.0), material_shininess);
    vec3 specular = spec * light_color * material_specular.rgb;
    
    // Combine lighting
    vec3 result = ambient + diffuse + specular;
    
    FragColor = vec4(result, material_alpha);
}
"#;

// Wireframe-specific shaders for line rendering
pub const WIREFRAME_VERTEX_SHADER: &str = r#"
#version 330 core

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec4 color;

uniform mat4 mvp_matrix;
uniform vec4 line_color;

out vec4 frag_color;

void main() {
    gl_Position = mvp_matrix * vec4(position, 1.0);
    frag_color = line_color;
}
"#;

pub const WIREFRAME_FRAGMENT_SHADER: &str = r#"
#version 330 core

in vec4 frag_color;
out vec4 FragColor;

void main() {
    FragColor = frag_color;
}
"#;