#!/usr/bin/env python3
"""
Generate simple STL test files for GCodeKit5 3D integration testing.
Creates basic geometric shapes as STL files.
"""

import struct
import os

def write_stl_binary(filename, triangles):
    """Write triangles to a binary STL file."""
    with open(filename, 'wb') as f:
        # Header (80 bytes)
        f.write(b'STL test file generated for GCodeKit5' + b' ' * 42)
        
        # Number of triangles (4 bytes)
        f.write(struct.pack('<I', len(triangles)))
        
        # Write each triangle
        for triangle in triangles:
            # Normal vector (12 bytes) - calculate from cross product
            v1, v2, v3 = triangle
            edge1 = [v2[i] - v1[i] for i in range(3)]
            edge2 = [v3[i] - v1[i] for i in range(3)]
            
            # Cross product for normal
            normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2], 
                edge1[0] * edge2[1] - edge1[1] * edge2[0]
            ]
            
            # Normalize
            length = sum(n*n for n in normal) ** 0.5
            if length > 0:
                normal = [n/length for n in normal]
            else:
                normal = [0, 0, 1]
            
            # Write normal
            f.write(struct.pack('<fff', *normal))
            
            # Write vertices (36 bytes)
            for vertex in triangle:
                f.write(struct.pack('<fff', *vertex))
            
            # Attribute byte count (2 bytes)
            f.write(b'\x00\x00')

def generate_cube_stl(size=10.0):
    """Generate a simple cube STL."""
    s = size / 2
    vertices = [
        [-s, -s, -s], [s, -s, -s], [s, s, -s], [-s, s, -s],  # bottom
        [-s, -s, s], [s, -s, s], [s, s, s], [-s, s, s]       # top
    ]
    
    # Define 12 triangles (2 per face)
    triangles = [
        # Bottom face
        [vertices[0], vertices[1], vertices[2]],
        [vertices[0], vertices[2], vertices[3]],
        # Top face  
        [vertices[4], vertices[6], vertices[5]],
        [vertices[4], vertices[7], vertices[6]],
        # Front face
        [vertices[0], vertices[4], vertices[5]],
        [vertices[0], vertices[5], vertices[1]],
        # Back face
        [vertices[2], vertices[6], vertices[7]],
        [vertices[2], vertices[7], vertices[3]],
        # Left face
        [vertices[0], vertices[3], vertices[7]],
        [vertices[0], vertices[7], vertices[4]],
        # Right face
        [vertices[1], vertices[5], vertices[6]],
        [vertices[1], vertices[6], vertices[2]]
    ]
    
    return triangles

def generate_pyramid_stl(base_size=10.0, height=8.0):
    """Generate a simple pyramid STL."""
    s = base_size / 2
    h = height
    
    # Base vertices
    base = [[-s, -s, 0], [s, -s, 0], [s, s, 0], [-s, s, 0]]
    apex = [0, 0, h]
    
    triangles = [
        # Base (2 triangles)
        [base[0], base[2], base[1]],
        [base[0], base[3], base[2]], 
        # Sides (4 triangles)
        [base[0], base[1], apex],
        [base[1], base[2], apex],
        [base[2], base[3], apex],
        [base[3], base[0], apex]
    ]
    
    return triangles

def generate_test_stl_files():
    """Generate test STL files in assets directory."""
    assets_dir = "assets"
    stl_dir = os.path.join(assets_dir, "test-stl")
    
    # Create directory if it doesn't exist
    os.makedirs(stl_dir, exist_ok=True)
    
    # Generate test files
    print("Generating test STL files...")
    
    # Simple cube
    cube_triangles = generate_cube_stl(10.0)
    write_stl_binary(os.path.join(stl_dir, "test_cube_10mm.stl"), cube_triangles)
    print(f"✅ Created test_cube_10mm.stl ({len(cube_triangles)} triangles)")
    
    # Small cube for detail testing
    small_cube = generate_cube_stl(2.0)
    write_stl_binary(os.path.join(stl_dir, "test_cube_2mm.stl"), small_cube)
    print(f"✅ Created test_cube_2mm.stl ({len(small_cube)} triangles)")
    
    # Pyramid
    pyramid_triangles = generate_pyramid_stl(8.0, 6.0)
    write_stl_binary(os.path.join(stl_dir, "test_pyramid_8x6mm.stl"), pyramid_triangles)
    print(f"✅ Created test_pyramid_8x6mm.stl ({len(pyramid_triangles)} triangles)")
    
    print(f"\nTest STL files created in: {stl_dir}")
    print("Files ready for GCodeKit5 3D integration testing!")

if __name__ == "__main__":
    generate_test_stl_files()