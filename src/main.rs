use raylib::prelude::*;
use std::f32::consts::PI;

mod framebuffer;
mod ray_intersect;
mod cube;
mod camera;
mod light;
mod material;
mod textures;

use framebuffer::Framebuffer;
use ray_intersect::{Intersect, RayIntersect};
use cube::Cube;
use camera::Camera;
use light::Light;
use material::{Material, vector3_to_color};
use textures::TextureManager;

const ORIGIN_BIAS: f32 = 1e-4;
const SKYBOX_COLOR: Vector3 = Vector3::new(0.26, 0.55, 0.89);

fn offset_origin(intersect: &Intersect, direction: &Vector3) -> Vector3 {
    let offset = intersect.normal * ORIGIN_BIAS;
    if direction.dot(intersect.normal) < 0.0 {
        intersect.point - offset
    } else {
        intersect.point + offset
    }
}

fn reflect(incident: &Vector3, normal: &Vector3) -> Vector3 {
    *incident - *normal * 2.0 * incident.dot(*normal)
}

fn refract(incident: &Vector3, normal: &Vector3, refractive_index: f32) -> Option<Vector3> {
    let mut cosi = incident.dot(*normal).max(-1.0).min(1.0);
    let mut etai = 1.0;
    let mut etat = refractive_index;
    let mut n = *normal;

    if cosi > 0.0 {
        std::mem::swap(&mut etai, &mut etat);
        n = -n;
    } else {
        cosi = -cosi;
    }

    let eta = etai / etat;
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);

    if k < 0.0 {
        None
    } else {
        Some(*incident * eta + n * (eta * cosi - k.sqrt()))
    }
}

fn cast_shadow(
    intersect: &Intersect,
    light: &Light,
    objects: &[Cube],
) -> f32 {
    let light_dir = (light.position - intersect.point).normalized();
    let light_distance = (light.position - intersect.point).length();

    let shadow_ray_origin = offset_origin(intersect, &light_dir);

    for object in objects {
        let shadow_intersect = object.ray_intersect(&shadow_ray_origin, &light_dir);
        if shadow_intersect.is_intersecting && shadow_intersect.distance < light_distance {
            return 1.0;
        }
    }

    0.0
}

pub fn cast_ray(
    ray_origin: &Vector3,
    ray_direction: &Vector3,
    objects: &[Cube],
    light: &Light,
    texture_manager: &TextureManager,
    depth: u32,
) -> Vector3 {
    if depth > 3 {
        return SKYBOX_COLOR;
    }

    let mut intersect = Intersect::empty();
    let mut zbuffer = f32::INFINITY;

    for object in objects {
        let i = object.ray_intersect(ray_origin, ray_direction);
        if i.is_intersecting && i.distance < zbuffer {
            zbuffer = i.distance;
            intersect = i;
        }
    }

    if !intersect.is_intersecting {
        return SKYBOX_COLOR;
    }

    let light_dir = (light.position - intersect.point).normalized();
    let view_dir = (*ray_origin - intersect.point).normalized();

    let mut normal = intersect.normal;
    if let Some(normal_map_path) = &intersect.material.normal_map_id {
        let texture = texture_manager.get_texture(normal_map_path).unwrap();
        let width = texture.width() as u32;
        let height = texture.height() as u32;
        let tx = (intersect.u * width as f32) as u32;
        let ty = (intersect.v * height as f32) as u32;

        if let Some(tex_normal) = texture_manager.get_normal_from_map(normal_map_path, tx, ty) {
            let tangent = Vector3::new(normal.y, -normal.x, 0.0).normalized();
            let bitangent = normal.cross(tangent);
            
            let transformed_normal_x = tex_normal.x * tangent.x + tex_normal.y * bitangent.x + tex_normal.z * normal.x;
            let transformed_normal_y = tex_normal.x * tangent.y + tex_normal.y * bitangent.y + tex_normal.z * normal.y;
            let transformed_normal_z = tex_normal.x * tangent.z + tex_normal.y * bitangent.z + tex_normal.z * normal.z;

            normal = Vector3::new(transformed_normal_x, transformed_normal_y, transformed_normal_z).normalized();
        }
    }

    let reflect_dir = reflect(&-light_dir, &normal).normalized();

    let shadow_intensity = cast_shadow(&intersect, light, objects);
    let light_intensity = light.intensity * (1.0 - shadow_intensity);

    let diffuse_color = if let Some(texture_path) = &intersect.material.texture_id {
        let texture = texture_manager.get_texture(texture_path).unwrap();
        let width = texture.width() as u32;
        let height = texture.height() as u32;
        let tx = (intersect.u * width as f32) as u32;
        let ty = (intersect.v * height as f32) as u32;
        let color = texture_manager.get_pixel_color(texture_path, tx, ty);
        color
    } else {
        intersect.material.diffuse
    };

    let diffuse_intensity = normal.dot(light_dir).max(0.0) * light_intensity;
    let diffuse = diffuse_color * diffuse_intensity;

    let specular_intensity = view_dir.dot(reflect_dir).max(0.0).powf(intersect.material.specular) * light_intensity;
    let light_color_v3 = Vector3::new(light.color.r as f32 / 255.0, light.color.g as f32 / 255.0, light.color.b as f32 / 255.0);
    let specular = light_color_v3 * specular_intensity;

    let albedo = intersect.material.albedo;
    let phong_color = diffuse * albedo[0] + specular * albedo[1];

    let reflectivity = intersect.material.albedo[2];
    let reflect_color = if reflectivity > 0.0 {
        let reflect_dir = reflect(ray_direction, &normal).normalized();
        let reflect_origin = offset_origin(&intersect, &reflect_dir);
        cast_ray(&reflect_origin, &reflect_dir, objects, light, texture_manager, depth + 1)
    } else {
        Vector3::zero()
    };

    let transparency = intersect.material.albedo[3];
    let refract_color = if transparency > 0.0 {
        if let Some(refract_dir) = refract(ray_direction, &normal, intersect.material.refractive_index) {
            let refract_origin = offset_origin(&intersect, &refract_dir);
            cast_ray(&refract_origin, &refract_dir, objects, light, texture_manager, depth + 1)
        } else {
            let reflect_dir = reflect(ray_direction, &normal).normalized();
            let reflect_origin = offset_origin(&intersect, &reflect_dir);
            cast_ray(&reflect_origin, &reflect_dir, objects, light, texture_manager, depth + 1)
        }
    } else {
        Vector3::zero()
    };

    phong_color * (1.0 - reflectivity - transparency) + reflect_color * reflectivity + refract_color * transparency
}

pub fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Cube],
    camera: &Camera,
    light: &Light,
    texture_manager: &TextureManager,
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    // Limpiar buffer con blit optimizado
    framebuffer.clear();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
            
            let rotated_direction = camera.basis_change(&ray_direction);

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, 0);
            let pixel_color = vector3_to_color(pixel_color_v3);

            framebuffer.set_current_color(pixel_color);
            framebuffer.set_pixel(x, y);
        }
    }
}

// Renderizado adaptativo con LOD (Level of Detail) suave y temporal accumulation
pub fn render_adaptive(
    framebuffer: &mut Framebuffer,
    objects: &[Cube],
    camera: &Camera,
    light: &Light,
    texture_manager: &TextureManager,
    lod_level: u32, // 1 = alta calidad, 4 = baja calidad
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    // No hacer clear si LOD es alto (para acumulación temporal)
    if lod_level >= 4 {
        framebuffer.clear();
    }

    let step_size = match lod_level {
        1 => 1, // Full resolution
        2 => 1, // Full resolution con jittering
        3 => 2, // Half resolution
        4 => 3, // Third resolution  
        _ => 4, // Quarter resolution o peor
    };

    // Patrón de jittering para mejor calidad visual
    let jitter_pattern = [(0, 0), (1, 0), (0, 1), (1, 1)];
    let jitter_offset = match lod_level {
        2 => Some(jitter_pattern[(lod_level as usize) % 4]),
        _ => None,
    };

    for y in (0..framebuffer.height).step_by(step_size) {
        for x in (0..framebuffer.width).step_by(step_size) {
            // Aplicar jitter para LOD 2
            let (actual_x, actual_y) = if let Some((jx, jy)) = jitter_offset {
                ((x + jx).min(framebuffer.width - 1), (y + jy).min(framebuffer.height - 1))
            } else {
                (x, y)
            };

            let screen_x = (2.0 * actual_x as f32) / width - 1.0;
            let screen_y = -(2.0 * actual_y as f32) / height + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
            let rotated_direction = camera.basis_change(&ray_direction);

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, 0);
            let pixel_color = vector3_to_color(pixel_color_v3);

            // Aplicar el color con estrategias diferentes según LOD
            match lod_level {
                1 => {
                    // Máxima calidad: pixel directo
                    framebuffer.set_current_color(pixel_color);
                    framebuffer.set_pixel(actual_x, actual_y);
                }
                2 => {
                    // Alta calidad con temporal blending
                    framebuffer.blend_pixel(actual_x, actual_y, pixel_color, 0.7);
                }
                3 => {
                    // Media calidad: llenar bloque 2x2
                    fill_adaptive_block(framebuffer, x, y, pixel_color, 2);
                }
                4 => {
                    // Baja calidad: llenar bloque 3x3
                    fill_adaptive_block(framebuffer, x, y, pixel_color, 3);
                }
                _ => {
                    // Muy baja calidad: llenar bloque grande
                    fill_adaptive_block(framebuffer, x, y, pixel_color, 4);
                }
            }
        }
    }
}

// Función mejorada para llenar bloques con degradado suave en los bordes
fn fill_adaptive_block(
    framebuffer: &mut Framebuffer,
    base_x: u32,
    base_y: u32,
    color: Color,
    size: usize,
) {
    framebuffer.set_current_color(color);
    
    for dy in 0..size {
        for dx in 0..size {
            let px = base_x + dx as u32;
            let py = base_y + dy as u32;
            
            if px < framebuffer.width && py < framebuffer.height {
                // Aplicar un ligero degradado en los bordes para suavizar
                let edge_factor = if dx == 0 || dy == 0 || dx == size-1 || dy == size-1 {
                    0.8 // Bordes ligeramente más oscuros
                } else {
                    1.0 // Centro completo
                };
                
                let adjusted_color = Color {
                    r: (color.r as f32 * edge_factor) as u8,
                    g: (color.g as f32 * edge_factor) as u8,
                    b: (color.b as f32 * edge_factor) as u8,
                    a: color.a,
                };
                
                framebuffer.set_current_color(adjusted_color);
                framebuffer.set_pixel(px, py);
            }
        }
    }
}

// Renderizado rápido a baja resolución para movimiento de cámara
pub fn render_fast(
    framebuffer: &mut Framebuffer,
    objects: &[Cube],
    camera: &Camera,
    light: &Light,
    texture_manager: &TextureManager,
    scale_factor: u32, // Factor de escala (2, 4, etc.)
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    framebuffer.clear();

    // Renderizar solo cada N píxeles y luego hacer upscale
    for y in (0..framebuffer.height).step_by(scale_factor as usize) {
        for x in (0..framebuffer.width).step_by(scale_factor as usize) {
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
            let rotated_direction = camera.basis_change(&ray_direction);

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, 0);
            let pixel_color = vector3_to_color(pixel_color_v3);

            framebuffer.set_current_color(pixel_color);
            
            // Llenar un bloque de píxeles con el mismo color (upscaling simple)
            for dy in 0..scale_factor {
                for dx in 0..scale_factor {
                    let px = x + dx;
                    let py = y + dy;
                    if px < framebuffer.width && py < framebuffer.height {
                        framebuffer.set_pixel(px, py);
                    }
                }
            }
        }
    }
}

// Renderizado progresivo para mejor rendimiento interactivo
pub fn render_progressive(
    framebuffer: &mut Framebuffer,
    objects: &[Cube],
    camera: &Camera,
    light: &Light,
    texture_manager: &TextureManager,
    samples_per_frame: u32,
    current_sample: &mut u32,
) -> bool {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    let total_pixels = framebuffer.width * framebuffer.height;
    
    // Solo limpiar si es el primer sample (evitar pantalla negra)
    if *current_sample == 0 {
        framebuffer.clear();
    }

    let start_pixel = *current_sample;
    let end_pixel = (start_pixel + samples_per_frame).min(total_pixels);

    // Renderizar solo la porción asignada
    for pixel_index in start_pixel..end_pixel {
        let x = pixel_index % framebuffer.width;
        let y = pixel_index / framebuffer.width;

        let screen_x = (2.0 * x as f32) / width - 1.0;
        let screen_y = -(2.0 * y as f32) / height + 1.0;

        let screen_x = screen_x * aspect_ratio * perspective_scale;
        let screen_y = screen_y * perspective_scale;

        let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
        let rotated_direction = camera.basis_change(&ray_direction);

        let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, 0);
        let pixel_color = vector3_to_color(pixel_color_v3);

        framebuffer.set_current_color(pixel_color);
        framebuffer.set_pixel(x, y);
    }

    *current_sample = end_pixel;
    
    // Retornar true si el renderizado está completo
    *current_sample >= total_pixels
}

fn main() {
    let window_width = 1300;
    let window_height = 900;
 
    let (mut window, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raytracer Example")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    let mut texture_manager = TextureManager::new();
    texture_manager.load_texture(&mut window, &thread, "assets/ball.png");
    texture_manager.load_texture(&mut window, &thread, "assets/ball_normal.png");
    texture_manager.load_texture(&mut window, &thread, "assets/bricks.png");
    texture_manager.load_texture(&mut window, &thread, "assets/bricks_normal.png");
    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);

    let rubber = Material::new(
        Vector3::new(0.3, 0.1, 0.1),
        10.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some("assets/ball.png".to_string()),
        Some("assets/ball_normal.png".to_string()),
    );

    let bricks = Material::new(
        Vector3::new(0.8, 0.2, 0.1),
        20.0,
        [0.8, 0.2, 0.0, 0.0],
        0.0,
        Some("assets/bricks.png".to_string()),
        Some("assets/bricks_normal.png".to_string()),
    );

    let ivory = Material::new(
        Vector3::new(0.4, 0.4, 0.3),
        50.0,
        [0.6, 0.3, 0.1, 0.0],
        0.0,
        None,
        None,
    );

    let glass = Material::new(
        Vector3::new(0.6, 0.7, 0.8),
        125.0,
        [0.0, 0.5, 0.1, 0.8],
        1.5,
        None,
        None,
    );

    // Crear un arreglo de cubos flotantes estilo skyblock
    let objects = [
        // Plataforma base
        Cube::new(Vector3::new(0.0, -2.0, 0.0), 2.0, bricks.clone()),
        Cube::new(Vector3::new(2.0, -2.0, 0.0), 2.0, bricks.clone()),
        Cube::new(Vector3::new(-2.0, -2.0, 0.0), 2.0, bricks.clone()),
        Cube::new(Vector3::new(0.0, -2.0, 2.0), 2.0, bricks.clone()),
        Cube::new(Vector3::new(0.0, -2.0, -2.0), 2.0, bricks.clone()),
        
        // Cubos flotantes a diferentes alturas
        Cube::new(Vector3::new(3.0, 0.0, 1.0), 1.5, rubber.clone()),
        Cube::new(Vector3::new(-3.0, 1.0, -1.0), 1.2, ivory.clone()),
        Cube::new(Vector3::new(1.0, 2.0, 3.0), 1.0, glass.clone()),
        Cube::new(Vector3::new(-1.5, 3.0, 2.0), 0.8, rubber.clone()),
        Cube::new(Vector3::new(2.5, 1.5, -2.5), 1.3, bricks.clone()),
        
        // Más cubos flotantes para efecto skyblock
        Cube::new(Vector3::new(-2.5, 4.0, 0.5), 1.0, ivory.clone()),
        Cube::new(Vector3::new(0.5, 5.0, -1.5), 0.7, glass.clone()),
        Cube::new(Vector3::new(4.0, 2.5, -0.5), 1.1, rubber.clone()),
        Cube::new(Vector3::new(-1.0, 6.0, 1.0), 0.6, bricks.clone()),
        Cube::new(Vector3::new(1.5, 3.5, 2.5), 0.9, ivory.clone()),
    ];

    let mut camera = Camera::new(
        Vector3::new(-5.0, 8.0, 8.0),
        Vector3::new(0.0, 2.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let rotation_speed = PI / 100.0;
    let zoom_speed = 0.1;

    // Variables para renderizado progresivo e híbrido
    let mut current_sample = 0u32;
    let samples_per_frame = (window_width * window_height / 120) as u32; // Más conservador
    let mut render_complete = false;
    let mut frames_since_camera_change = 0u32;
    let mut use_progressive = false;
    let mut current_lod = 4u32; // Level of Detail inicial (más bajo = mejor calidad)
    let mut target_lod = 1u32;

    let light = Light::new(
        Vector3::new(5.0, 10.0, 5.0),
        Color::new(255, 255, 255, 255),
        2.0,
    );

    while !window.window_should_close() {
        let camera_was_changed = camera.is_changed();
        
        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            camera.orbit(rotation_speed, 0.0);
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            camera.orbit(-rotation_speed, 0.0);
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            camera.orbit(0.0, -rotation_speed);
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            camera.orbit(0.0, rotation_speed);
        }
        if window.is_key_down(KeyboardKey::KEY_W) {
            camera.zoom(zoom_speed);
        }
        if window.is_key_down(KeyboardKey::KEY_S) {
            camera.zoom(-zoom_speed);
        }

        // Lógica híbrida mejorada con LOD adaptativo
        if camera_was_changed {
            frames_since_camera_change = 0;
            use_progressive = false;
            current_lod = 4; // Empezar con baja calidad
            target_lod = 1; // Objetivo: alta calidad
            render_complete = false;
        }
        
        // Ajustar LOD gradualmente para transición más suave (cada 2 frames)
        if frames_since_camera_change % 2 == 0 && current_lod > target_lod {
            current_lod = (current_lod - 1).max(target_lod);
        }
        
        frames_since_camera_change += 1;
        
        // Renderizado adaptativo basado en frames y LOD
        if frames_since_camera_change <= 8 {
            // Fase inicial: renderizado adaptativo con mejora gradual
            render_adaptive(&mut framebuffer, &objects, &camera, &light, &texture_manager, current_lod);
        } else if frames_since_camera_change <= 20 {
            // Fase intermedia: renderizado completo si no está hecho
            if !render_complete {
                render(&mut framebuffer, &objects, &camera, &light, &texture_manager);
                render_complete = true;
            }
        } else {
            // Fase final: renderizado progresivo de alta calidad
            if !use_progressive {
                current_sample = 0;
                render_complete = false;
                use_progressive = true;
            }
            
            if !render_complete {
                render_complete = render_progressive(
                    &mut framebuffer, 
                    &objects, 
                    &camera, 
                    &light, 
                    &texture_manager, 
                    samples_per_frame,
                    &mut current_sample
                );
            }
        }
        
        // Usar el sistema optimizado de blit y caché
        framebuffer.swap_buffers(&mut window, &thread);
    }
}
