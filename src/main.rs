use raylib::prelude::*;
use std::f32::consts::PI;

mod framebuffer;
mod ray_intersect;
mod cube;
mod camera;
mod light;
mod material;
mod textures;
mod skybox;

use framebuffer::Framebuffer;
use ray_intersect::{Intersect, RayIntersect};
use cube::Cube;
use camera::Camera;
use light::Light;
use material::{Material, vector3_to_color};
use textures::TextureManager;
use skybox::Skybox;

const ORIGIN_BIAS: f32 = 1e-4;

// ========== SISTEMA DE ROTACIÓN GLOBAL DE ESCENA ==========
#[derive(Clone, Copy)]
struct Matrix3 {
    data: [[f32; 3]; 3],
}

impl Matrix3 {
    fn rotation_y(angle: f32) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Matrix3 {
            data: [
                [cos_a, 0.0, sin_a],
                [0.0, 1.0, 0.0],
                [-sin_a, 0.0, cos_a],
            ]
        }
    }

    fn transform_vector(&self, v: Vector3) -> Vector3 {
        Vector3::new(
            self.data[0][0] * v.x + self.data[0][1] * v.y + self.data[0][2] * v.z,
            self.data[1][0] * v.x + self.data[1][1] * v.y + self.data[1][2] * v.z,
            self.data[2][0] * v.x + self.data[2][1] * v.y + self.data[2][2] * v.z,
        )
    }
}

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
    skybox: &Skybox,
    depth: u32,
) -> Vector3 {
    if depth > 3 {
        return skybox.get_color(ray_direction);
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
        return skybox.get_color(ray_direction);
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
        cast_ray(&reflect_origin, &reflect_dir, objects, light, texture_manager, skybox, depth + 1)
    } else {
        Vector3::zero()
    };

    let transparency = intersect.material.albedo[3];
    let refract_color = if transparency > 0.0 {
        if let Some(refract_dir) = refract(ray_direction, &normal, intersect.material.refractive_index) {
            let refract_origin = offset_origin(&intersect, &refract_dir);
            cast_ray(&refract_origin, &refract_dir, objects, light, texture_manager, skybox, depth + 1)
        } else {
            let reflect_dir = reflect(ray_direction, &normal).normalized();
            let reflect_origin = offset_origin(&intersect, &reflect_dir);
            cast_ray(&reflect_origin, &reflect_dir, objects, light, texture_manager, skybox, depth + 1)
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
    skybox: &Skybox,
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

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, skybox, 0);
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
    skybox: &Skybox,
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

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, skybox, 0);
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
    skybox: &Skybox,
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

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, skybox, 0);
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
    skybox: &Skybox,
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

        let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, skybox, 0);
        let pixel_color = vector3_to_color(pixel_color_v3);

        framebuffer.set_current_color(pixel_color);
        framebuffer.set_pixel(x, y);
    }

    *current_sample = end_pixel;
    
    // Retornar true si el renderizado está completo
    *current_sample >= total_pixels
}

// ========== FUNCIONES DE TRANSFORMACIÓN GLOBAL ==========
fn create_rotated_objects(base_objects: &[Cube], scene_rotation_angle: f32) -> Vec<Cube> {
    // Optimización: calcular la matriz una sola vez
    let rotation_matrix = Matrix3::rotation_y(scene_rotation_angle);
    
    // Pre-reservar el vector para evitar realocaciones
    let mut rotated_objects = Vec::with_capacity(base_objects.len());
    
    // Aplicar la rotación usando iterador para mejor rendimiento
    for object in base_objects {
        let mut rotated_object = object.clone();
        rotated_object.center = rotation_matrix.transform_vector(object.center);
        rotated_objects.push(rotated_object);
    }
    
    rotated_objects
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
    // Texturas antiguas (comentadas porque no existen)
    // texture_manager.load_texture(&mut window, &thread, "assets/ball.png");
    // texture_manager.load_texture(&mut window, &thread, "assets/ball_normal.png");
    // texture_manager.load_texture(&mut window, &thread, "assets/bricks.png");
    // texture_manager.load_texture(&mut window, &thread, "assets/bricks_normal.png");
    
    // Cargar texturas nuevas que sí existen
    texture_manager.load_texture(&mut window, &thread, "assets/grass_dirt.png");
    texture_manager.load_texture(&mut window, &thread, "assets/grass_dirt_normal.png");
    texture_manager.load_texture(&mut window, &thread, "assets/castle_stone.png");
    texture_manager.load_texture(&mut window, &thread, "assets/castle_stone_normal.png");
    texture_manager.load_texture(&mut window, &thread, "assets/water_waves.png");
    texture_manager.load_texture(&mut window, &thread, "assets/water_normal.png");
    texture_manager.load_texture(&mut window, &thread, "assets/lava_bubbles.png");
    texture_manager.load_texture(&mut window, &thread, "assets/lava_normal.png");
    
    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);

    // ========== CREAR SKYBOX ==========
    // Skybox atmosférico con atardecer (puedes cambiar por otros presets)
    let mut skybox = Skybox::sunset(); // También puedes usar: midday(), night(), overcast(), cosmic()
    
    // Materiales temáticos usando las texturas disponibles
    let tierra_hierba = Material::tierra_hierba(); // Ya tiene las rutas correctas
    let piedra_castillo = Material::piedra_castillo(); // Ya tiene las rutas correctas
    let agua = Material::agua(); // Ya tiene las rutas correctas
    let lava = Material::lava(); // Ya tiene las rutas correctas
    let cristal_blanco = Material::cristal_gema();
    let cristal_esmeralda = Material::cristal_esmeralda();
    let cristal_rubi = Material::cristal_rubi();
    let cristal_zafiro = Material::cristal_zafiro();
    
    // Nuevos materiales para elementos del diorama
    let madera = Material::madera();
    let hojas = Material::hojas();
    let piedra_oscura = Material::piedra_oscura();

    // Crear un diorama de terreno flotante con cuadrícula 5x5
    let base_objects = [
        // ========== TERRENO BASE - CUADRÍCULA 5x5 ==========
        // Fila trasera (Z = -4) - Elevación alta para montañas
        Cube::new(Vector3::new(-4.0, -1.0, -4.0), 2.0, tierra_hierba.clone()), // Esquina noroeste
        Cube::new(Vector3::new(-2.0, 0.0, -4.0), 2.0, tierra_hierba.clone()),  // Elevación media
        Cube::new(Vector3::new(0.0, 1.0, -4.0), 2.0, tierra_hierba.clone()),   // Pico montañoso
        Cube::new(Vector3::new(2.0, 0.5, -4.0), 2.0, tierra_hierba.clone()),   // Descendiendo
        Cube::new(Vector3::new(4.0, -0.5, -4.0), 2.0, tierra_hierba.clone()),  // Esquina noreste
        
        // Fila medio-trasera (Z = -2) - Transición de montaña a valle
        Cube::new(Vector3::new(-4.0, -1.5, -2.0), 2.0, tierra_hierba.clone()), // Ladera oeste
        Cube::new(Vector3::new(-2.0, -1.0, -2.0), 2.0, tierra_hierba.clone()), // Valle intermedio
        Cube::new(Vector3::new(0.0, 0.0, -2.0), 2.0, tierra_hierba.clone()),   // Planicie central
        Cube::new(Vector3::new(2.0, -0.5, -2.0), 2.0, tierra_hierba.clone()),  // Inicio descenso
        Cube::new(Vector3::new(4.0, -1.0, -2.0), 2.0, tierra_hierba.clone()),  // Ladera este
        
        // Fila central (Z = 0) - Nivel principal del diorama
        Cube::new(Vector3::new(-4.0, -2.0, 0.0), 2.0, tierra_hierba.clone()),  // Nivel bajo oeste
        Cube::new(Vector3::new(-2.0, -1.5, 0.0), 2.0, tierra_hierba.clone()),  // Río: orilla oeste
        // [ESPACIO PARA RÍO] Vector3::new(0.0, -3.0, 0.0) - Cauce del río
        Cube::new(Vector3::new(2.0, -1.5, 0.0), 2.0, tierra_hierba.clone()),   // Río: orilla este
        Cube::new(Vector3::new(4.0, -2.0, 0.0), 2.0, tierra_hierba.clone()),   // Nivel bajo este
        
        // Fila medio-frontal (Z = 2) - Área de cascada
        Cube::new(Vector3::new(-4.0, -2.5, 2.0), 2.0, tierra_hierba.clone()),  // Terraza baja
        Cube::new(Vector3::new(-2.0, -2.0, 2.0), 2.0, tierra_hierba.clone()),  // Cascada: nivel alto
        Cube::new(Vector3::new(0.0, -3.5, 2.0), 2.0, tierra_hierba.clone()),   // Piscina de cascada
        Cube::new(Vector3::new(2.0, -2.0, 2.0), 2.0, tierra_hierba.clone()),   // Terraza este
        Cube::new(Vector3::new(4.0, -2.5, 2.0), 2.0, tierra_hierba.clone()),   // Borde sur-este
        
        // Fila frontal (Z = 4) - Nivel más bajo
        Cube::new(Vector3::new(-4.0, -3.0, 4.0), 2.0, tierra_hierba.clone()),  // Base suroeste
        Cube::new(Vector3::new(-2.0, -3.0, 4.0), 2.0, tierra_hierba.clone()),  // Valle sur
        Cube::new(Vector3::new(0.0, -4.0, 4.0), 2.0, tierra_hierba.clone()),   // Punto más bajo
        Cube::new(Vector3::new(2.0, -3.0, 4.0), 2.0, tierra_hierba.clone()),   // Valle sur-este
        Cube::new(Vector3::new(4.0, -3.0, 4.0), 2.0, tierra_hierba.clone()),   // Base sureste
        
        // ========== SISTEMA DE AGUA COMPLEJO - RÍO Y CASCADA ==========
        // Nacimiento del río (manantial en las montañas)
        Cube::new(Vector3::new(0.0, 0.5, -3.5), 0.8, agua.clone()),     // Manantial montañoso
        Cube::new(Vector3::new(0.0, 0.2, -3.0), 1.0, agua.clone()),     // Primera poza
        Cube::new(Vector3::new(0.0, -0.2, -2.5), 1.1, agua.clone()),    // Flujo inicial descendente
        
        // Río principal serpenteante (fluye de norte a sur)
        Cube::new(Vector3::new(-0.3, -1.0, -1.8), 1.2, agua.clone()),   // Meandro oeste 1
        Cube::new(Vector3::new(0.2, -1.5, -1.2), 1.1, agua.clone()),    // Meandro este 1
        Cube::new(Vector3::new(-0.2, -2.0, -0.6), 1.3, agua.clone()),   // Meandro oeste 2
        Cube::new(Vector3::new(0.0, -2.5, 0.0), 1.4, agua.clone()),     // Cauce central principal
        Cube::new(Vector3::new(0.1, -2.8, 0.5), 1.3, agua.clone()),     // Pre-cascada este
        Cube::new(Vector3::new(-0.1, -3.0, 1.0), 1.2, agua.clone()),    // Pre-cascada oeste
        
        // Sistema de cascada múltiple (caídas escalonadas)
        Cube::new(Vector3::new(0.0, -2.3, 1.5), 1.1, agua.clone()),     // Nivel superior cascada
        Cube::new(Vector3::new(0.0, -2.8, 1.8), 0.9, agua.clone()),     // Primera caída
        Cube::new(Vector3::new(0.0, -3.5, 2.1), 1.0, agua.clone()),     // Poza intermedia
        Cube::new(Vector3::new(0.0, -4.0, 2.4), 0.8, agua.clone()),     // Segunda caída
        Cube::new(Vector3::new(0.0, -4.7, 2.7), 1.2, agua.clone()),     // Tercera caída
        Cube::new(Vector3::new(0.0, -5.2, 3.0), 1.6, agua.clone()),     // Piscina mayor inferior
        
        // Extensión del río después de la cascada
        Cube::new(Vector3::new(0.0, -5.4, 3.5), 1.4, agua.clone()),     // Continuación río
        Cube::new(Vector3::new(-0.2, -5.5, 4.0), 1.3, agua.clone()),    // Meandro final oeste
        Cube::new(Vector3::new(0.3, -5.6, 4.5), 1.2, agua.clone()),     // Salida este del río
        
        // Afluentes secundarios (tributarios)
        Cube::new(Vector3::new(-1.5, -1.8, -1.0), 0.8, agua.clone()),   // Afluente oeste 1
        Cube::new(Vector3::new(-1.0, -2.2, -0.5), 0.9, agua.clone()),   // Confluencia oeste
        Cube::new(Vector3::new(1.8, -1.9, 0.8), 0.7, agua.clone()),     // Afluente este 1
        Cube::new(Vector3::new(1.3, -2.4, 0.3), 0.8, agua.clone()),     // Confluencia este
        
        // Lagos y pozas adicionales
        Cube::new(Vector3::new(-2.5, -2.8, 1.2), 1.0, agua.clone()),    // Lago oeste
        Cube::new(Vector3::new(2.8, -3.2, 1.8), 1.1, agua.clone()),     // Lago este
        Cube::new(Vector3::new(-1.2, -4.8, 3.8), 0.9, agua.clone()),    // Poza de remanso oeste
        Cube::new(Vector3::new(1.5, -5.0, 4.2), 0.8, agua.clone()),     // Poza de remanso este
        
        // ========== ESTRUCTURAS DE CASTILLO MEJORADAS ==========
        // Fundaciones del castillo (sobre el pico montañoso Y=1.0)
        Cube::new(Vector3::new(0.0, 1.5, -4.0), 1.8, piedra_castillo.clone()),  // Fundación central
        Cube::new(Vector3::new(-1.0, 1.2, -4.0), 1.2, piedra_castillo.clone()), // Fundación oeste
        Cube::new(Vector3::new(1.0, 1.2, -4.0), 1.2, piedra_castillo.clone()),  // Fundación este
        
        // Torre principal (construida sobre las fundaciones)
        Cube::new(Vector3::new(0.0, 2.8, -4.0), 1.5, piedra_castillo.clone()),  // Base torre (Y=1.5+1.3=2.8)
        Cube::new(Vector3::new(0.0, 4.0, -4.0), 1.2, piedra_castillo.clone()),  // Torre media
        Cube::new(Vector3::new(0.0, 5.0, -4.0), 0.8, piedra_castillo.clone()),  // Torre alta
        
        // Murallas del castillo (sobre terreno base)
        Cube::new(Vector3::new(-1.5, 1.5, -3.5), 1.0, piedra_castillo.clone()), // Muralla oeste (sobre Y=0.0 + 1.5)
        Cube::new(Vector3::new(1.5, 1.5, -3.5), 1.0, piedra_castillo.clone()),  // Muralla este
        Cube::new(Vector3::new(0.0, 1.0, -3.0), 1.5, piedra_castillo.clone()),  // Muralla frontal
        
        // Torres de las esquinas (con bases sólidas)
        Cube::new(Vector3::new(-2.0, 0.8, -3.0), 1.2, piedra_castillo.clone()), // Base torre suroeste
        Cube::new(Vector3::new(-2.0, 2.0, -3.0), 1.0, piedra_castillo.clone()), // Torre suroeste
        Cube::new(Vector3::new(2.0, 0.8, -3.0), 1.2, piedra_castillo.clone()),  // Base torre sureste
        Cube::new(Vector3::new(2.0, 2.0, -3.0), 1.0, piedra_castillo.clone()),  // Torre sureste
        
        // Puertas y accesos (a nivel del suelo)
        Cube::new(Vector3::new(-0.8, 0.5, -2.8), 0.6, piedra_castillo.clone()), // Entrada oeste
        Cube::new(Vector3::new(0.8, 0.5, -2.8), 0.6, piedra_castillo.clone()),  // Entrada este
        
        // ========== RUINAS ANTIGUAS (SOBRE TERRENO) ==========
        // Ruinas en el lado oeste (sobre ladera Y=-1.5)
        Cube::new(Vector3::new(-3.5, -1.0, -1.0), 0.8, piedra_oscura.clone()), // Pilar en ruinas (sobre terreno)
        Cube::new(Vector3::new(-3.2, -0.5, -0.8), 0.6, piedra_oscura.clone()), // Fragmento superior
        Cube::new(Vector3::new(-4.0, -1.3, -0.5), 0.7, piedra_oscura.clone()), // Base de ruina (sobre terreno Y=-2.0)
        Cube::new(Vector3::new(-3.8, -0.8, 0.2), 0.5, piedra_oscura.clone()),  // Fragmento caído
        
        // Ruinas junto al río (sobre terreno Y=-2.5)
        Cube::new(Vector3::new(-2.5, -2.3, 1.5), 0.7, piedra_oscura.clone()),  // Ruina sobre terraza
        Cube::new(Vector3::new(-2.2, -2.0, 1.8), 0.4, piedra_oscura.clone()),  // Fragmento pequeño
        
        // ========== BOSQUE Y ÁRBOLES (PLANTADOS EN TERRENO) ==========
        // Árbol grande en la ladera oeste (sobre terreno Y=-1.5)
        Cube::new(Vector3::new(-3.5, -1.3, 0.5), 0.4, madera.clone()),   // Tronco base (plantado en terreno)
        Cube::new(Vector3::new(-3.5, -0.9, 0.5), 0.4, madera.clone()),   // Tronco medio
        Cube::new(Vector3::new(-3.5, -0.5, 0.5), 0.3, madera.clone()),   // Tronco superior
        Cube::new(Vector3::new(-3.5, -0.1, 0.5), 1.2, hojas.clone()),    // Copa del árbol
        Cube::new(Vector3::new(-3.2, 0.1, 0.8), 0.8, hojas.clone()),     // Rama este
        Cube::new(Vector3::new(-3.8, 0.1, 0.2), 0.8, hojas.clone()),     // Rama oeste
        
        // Grupo de árboles pequeños (sobre terraza Y=-2.5)
        Cube::new(Vector3::new(-2.8, -2.3, 1.8), 0.3, madera.clone()),   // Tronco 1 (plantado)
        Cube::new(Vector3::new(-2.8, -1.8, 1.8), 0.7, hojas.clone()),    // Copa 1
        Cube::new(Vector3::new(-2.2, -2.4, 2.2), 0.25, madera.clone()),  // Tronco 2 (plantado)
        Cube::new(Vector3::new(-2.2, -2.0, 2.2), 0.6, hojas.clone()),    // Copa 2
        Cube::new(Vector3::new(-2.5, -2.2, 2.5), 0.3, madera.clone()),   // Tronco 3 (plantado)
        Cube::new(Vector3::new(-2.5, -1.7, 2.5), 0.8, hojas.clone()),    // Copa 3
        
        // Árbol junto al río (sobre orilla Y=-1.5)
        Cube::new(Vector3::new(1.8, -1.3, 0.2), 0.35, madera.clone()),   // Tronco sauce (plantado)
        Cube::new(Vector3::new(1.8, -0.9, 0.2), 0.3, madera.clone()),    // Tronco medio
        Cube::new(Vector3::new(1.8, -0.5, 0.2), 1.0, hojas.clone()),     // Copa sauce
        Cube::new(Vector3::new(1.5, -0.7, 0.5), 0.7, hojas.clone()),     // Ramas colgantes
        Cube::new(Vector3::new(2.1, -0.8, -0.1), 0.6, hojas.clone()),    // Más ramas
        
        // Árboles en las montañas (sobre elevación Y=0.5)
        Cube::new(Vector3::new(2.2, 0.7, -3.5), 0.3, madera.clone()),    // Tronco montaña (plantado)
        Cube::new(Vector3::new(2.2, 1.3, -3.5), 0.9, hojas.clone()),     // Copa montaña
        Cube::new(Vector3::new(-1.8, 0.2, -3.2), 0.25, madera.clone()),  // Tronco pequeño (plantado)
        Cube::new(Vector3::new(-1.8, 0.6, -3.2), 0.7, hojas.clone()),    // Copa pequeña
        
        // ========== SISTEMA DE LAVA COMPLEJO ==========
        // Volcán principal (fuente de lava en el noreste)
        Cube::new(Vector3::new(4.0, -0.8, -4.0), 1.2, lava.clone()),    // Cráter volcánico
        Cube::new(Vector3::new(4.0, 0.0, -4.0), 1.0, lava.clone()),     // Boca del volcán
        Cube::new(Vector3::new(4.0, 0.8, -4.0), 0.8, lava.clone()),     // Erupción menor
        Cube::new(Vector3::new(4.0, 1.5, -4.0), 0.6, lava.clone()),     // Pico eruptivo
        
        // Flujo principal de lava (desde volcán hacia abajo)
        Cube::new(Vector3::new(3.8, -1.0, -3.5), 1.1, lava.clone()),    // Inicio flujo norte
        Cube::new(Vector3::new(3.6, -1.3, -3.0), 1.2, lava.clone()),    // Flujo descendente 1
        Cube::new(Vector3::new(3.4, -1.6, -2.5), 1.3, lava.clone()),    // Flujo descendente 2
        Cube::new(Vector3::new(3.2, -1.9, -2.0), 1.4, lava.clone()),    // Flujo principal medio
        Cube::new(Vector3::new(3.0, -2.2, -1.5), 1.3, lava.clone()),    // Continuación flujo
        Cube::new(Vector3::new(2.8, -2.5, -1.0), 1.2, lava.clone()),    // Flujo medio
        Cube::new(Vector3::new(2.6, -2.8, -0.5), 1.1, lava.clone()),    // Flujo bajo
        
        // Ramificaciones del flujo de lava
        // Rama este del flujo
        Cube::new(Vector3::new(3.5, -2.0, -1.8), 0.9, lava.clone()),    // Rama este 1
        Cube::new(Vector3::new(3.8, -2.4, -1.3), 0.8, lava.clone()),    // Rama este 2
        Cube::new(Vector3::new(4.1, -2.8, -0.8), 0.9, lava.clone()),    // Rama este 3
        Cube::new(Vector3::new(4.3, -3.2, -0.3), 1.0, lava.clone()),    // Poza de lava este
        
        // Rama oeste del flujo
        Cube::new(Vector3::new(2.8, -2.3, -1.2), 0.8, lava.clone()),    // Rama oeste 1
        Cube::new(Vector3::new(2.4, -2.7, -0.7), 0.9, lava.clone()),    // Rama oeste 2
        Cube::new(Vector3::new(2.0, -3.1, -0.2), 1.0, lava.clone()),    // Rama oeste 3
        Cube::new(Vector3::new(1.6, -3.5, 0.3), 1.1, lava.clone()),     // Poza de lava oeste
        
        // Lagos de lava (acumulaciones)
        Cube::new(Vector3::new(3.5, -3.0, 0.0), 1.3, lava.clone()),     // Lago de lava central
        Cube::new(Vector3::new(3.8, -3.4, 0.8), 1.2, lava.clone()),     // Lago de lava sur-este
        Cube::new(Vector3::new(2.2, -3.8, 0.9), 1.1, lava.clone()),     // Lago de lava sur-oeste
        
        // Flujos secundarios (corrientes menores)
        Cube::new(Vector3::new(3.0, -3.5, 1.2), 0.9, lava.clone()),     // Corriente secundaria 1
        Cube::new(Vector3::new(3.2, -4.0, 1.8), 0.8, lava.clone()),     // Corriente secundaria 2
        Cube::new(Vector3::new(2.8, -4.2, 2.2), 0.7, lava.clone()),     // Corriente final
        
        // Vents volcánicos adicionales (respiraderos menores)
        Cube::new(Vector3::new(3.5, -1.5, -4.2), 0.7, lava.clone()),    // Vent secundario 1
        Cube::new(Vector3::new(4.3, -1.2, -3.8), 0.6, lava.clone()),    // Vent secundario 2
        Cube::new(Vector3::new(3.7, 0.3, -3.6), 0.5, lava.clone()),     // Vent menor 1
        Cube::new(Vector3::new(4.2, 0.5, -3.9), 0.4, lava.clone()),     // Vent menor 2
        
        // Pozas de enfriamiento (lava más oscura/solidificándose)
        Cube::new(Vector3::new(2.5, -4.5, 2.8), 1.0, lava.clone()),     // Poza de enfriamiento 1
        Cube::new(Vector3::new(3.0, -4.8, 3.5), 1.1, lava.clone()),     // Poza de enfriamiento 2
        Cube::new(Vector3::new(2.0, -5.0, 3.2), 0.9, lava.clone()),     // Poza final
        
        // ========== INTERACCIÓN AGUA-LAVA (ZONA DE CONFLICTO) ==========
        // Área donde lava y agua se encuentran (vapor y efectos)
        Cube::new(Vector3::new(1.8, -4.5, 2.5), 0.6, agua.clone()),     // Agua resistiendo lava
        Cube::new(Vector3::new(2.2, -4.3, 2.3), 0.5, lava.clone()),     // Lava encuentro agua
        Cube::new(Vector3::new(2.0, -4.0, 2.4), 0.4, cristal_blanco.clone()), // Vapor/cristalización
        
        // Zona de batalla termal
        Cube::new(Vector3::new(1.5, -4.8, 3.0), 0.7, agua.clone()),     // Agua defendiendo
        Cube::new(Vector3::new(2.3, -4.6, 2.9), 0.6, lava.clone()),     // Lava avanzando
        Cube::new(Vector3::new(1.9, -4.2, 2.95), 0.3, cristal_blanco.clone()), // Cristalización vapor
        
        // ========== EFECTOS ADICIONALES DE FLUIDOS ==========
        // Salpicaduras y gotas de agua (cerca de cascadas)
        Cube::new(Vector3::new(-0.5, -3.5, 2.0), 0.3, agua.clone()),    // Salpicadura oeste cascada
        Cube::new(Vector3::new(0.6, -3.8, 2.2), 0.25, agua.clone()),    // Salpicadura este cascada
        Cube::new(Vector3::new(-0.3, -4.2, 2.8), 0.2, agua.clone()),    // Gota de agua 1
        Cube::new(Vector3::new(0.4, -4.5, 3.1), 0.2, agua.clone()),     // Gota de agua 2
        
        // Salpicaduras de lava (erupciones menores)
        Cube::new(Vector3::new(3.8, 0.8, -3.2), 0.3, lava.clone()),     // Salpicadura volcán 1
        Cube::new(Vector3::new(4.2, 1.2, -3.5), 0.25, lava.clone()),    // Salpicadura volcán 2
        Cube::new(Vector3::new(3.6, 1.0, -3.8), 0.2, lava.clone()),     // Proyectil lava 1
        Cube::new(Vector3::new(4.1, 1.5, -3.0), 0.2, lava.clone()),     // Proyectil lava 2
        
        // Estanques de reflexión perfecta (agua muy tranquila)
        Cube::new(Vector3::new(-3.5, -2.5, 0.5), 1.0, agua.clone()),    // Estanque espejo oeste
        Cube::new(Vector3::new(3.2, -4.2, 4.0), 1.2, agua.clone()),     // Estanque espejo este
        
        // Fuentes termales (donde lava calienta agua subterránea)
        Cube::new(Vector3::new(1.8, -3.8, -0.8), 0.8, agua.clone()),    // Fuente termal 1
        Cube::new(Vector3::new(2.5, -3.5, -1.2), 0.7, agua.clone()),    // Fuente termal 2
        Cube::new(Vector3::new(2.1, -3.2, -1.0), 0.4, cristal_blanco.clone()), // Vapor termal
        
        
        // ========== FORMACIONES CRISTALINAS MEJORADAS ==========
        // Cueva de cristales en el este (sobre terreno Y=-2.0)
        Cube::new(Vector3::new(3.5, -1.8, 0.5), 1.2, cristal_esmeralda.clone()), // Cristal madre (plantado)
        Cube::new(Vector3::new(3.8, -1.0, 0.8), 0.8, cristal_esmeralda.clone()), // Cristal hijo 1
        Cube::new(Vector3::new(3.2, -1.2, 0.2), 0.9, cristal_esmeralda.clone()), // Cristal hijo 2
        Cube::new(Vector3::new(3.6, -0.5, 0.6), 0.6, cristal_esmeralda.clone()), // Cristal pequeño
        Cube::new(Vector3::new(3.4, 0.0, 0.4), 0.4, cristal_blanco.clone()),     // Cristal punta
        
        // Formación de cristales de fuego (cerca de la lava, sobre Y=-1.0)
        Cube::new(Vector3::new(3.8, -0.8, -1.5), 0.7, cristal_rubi.clone()),     // Cristal de fuego base
        Cube::new(Vector3::new(4.1, -0.2, -1.3), 0.5, cristal_rubi.clone()),     // Cristal ardiente
        Cube::new(Vector3::new(3.9, 0.0, -1.7), 0.6, cristal_rubi.clone()),      // Cristal lateral
        Cube::new(Vector3::new(4.2, 0.3, -1.4), 0.3, cristal_blanco.clone()),    // Cristal caliente
        
        // Cristales de agua (cerca de la cascada, sobre terraza Y=-2.5)
        Cube::new(Vector3::new(0.8, -2.3, 2.8), 0.6, cristal_zafiro.clone()),    // Cristal acuático (plantado)
        Cube::new(Vector3::new(0.5, -1.8, 3.2), 0.8, cristal_zafiro.clone()),    // Cristal de cascada
        Cube::new(Vector3::new(1.2, -2.0, 3.0), 0.4, cristal_blanco.clone()),    // Cristal de espuma
        Cube::new(Vector3::new(0.3, -1.5, 3.5), 0.5, cristal_zafiro.clone()),    // Cristal junto a cascada
        
        // Cristales flotantes mágicos (solo estos pueden flotar - son mágicos)
        Cube::new(Vector3::new(-3.0, 4.0, -1.0), 0.6, cristal_esmeralda.clone()), // Cristal del bosque (más bajo)
        Cube::new(Vector3::new(3.0, 2.5, 1.0), 0.5, cristal_rubi.clone()),        // Cristal del fuego
        Cube::new(Vector3::new(-1.0, 3.5, 3.0), 0.7, cristal_zafiro.clone()),     // Cristal del agua
        Cube::new(Vector3::new(0.0, 6.0, -2.0), 0.4, cristal_blanco.clone()),     // Cristal del cielo (reducido)
        Cube::new(Vector3::new(2.5, 4.5, 0.5), 0.5, cristal_esmeralda.clone()),   // Cristal errante
        
        // Cristales mágicos adicionales (flotantes pero más bajos)
        Cube::new(Vector3::new(-1.5, 5.0, 1.0), 0.3, cristal_rubi.clone()),       // Cristal rubí flotante
        Cube::new(Vector3::new(1.8, 4.0, -1.5), 0.4, cristal_zafiro.clone()),     // Cristal zafiro medio
        Cube::new(Vector3::new(-0.5, 4.8, 2.5), 0.35, cristal_blanco.clone()),    // Cristal puro flotante
    ];

    // ========== SISTEMA DE ROTACIÓN GLOBAL DE ESCENA ==========
    let mut scene_rotation_angle = 0.0f32;
    let mut scene_rotation_speed = 0.0f32; // Radianes por frame
    let rotation_speed_increment = 0.001f32;
    let max_rotation_speed = 0.05f32;
    
    // ========== SISTEMA DE ZOOM AVANZADO ==========
    let mut zoom_speed = 0.1f32;
    let min_zoom_speed = 0.05f32;
    let max_zoom_speed = 0.5f32;
    let zoom_speed_increment = 0.05f32;

    // Ajustar cámara para vista frontal del diorama
    let mut camera = Camera::new(
        Vector3::new(0.0, 0.0, 15.0),   // Posición frontal: centrada en X, elevada en Y, alejada en Z
        Vector3::new(0.0, 0.0, 0.0),    // Mirar al centro del diorama
        Vector3::new(0.0, 1.0, 0.0),    // Vector up estándar
    );
    let rotation_speed = PI / 100.0;

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
        // ========== ACTUALIZACIÓN DE ROTACIÓN GLOBAL ==========
        scene_rotation_angle += scene_rotation_speed;
        
        // Optimización: solo crear objetos rotados si hay rotación
        let objects = if scene_rotation_angle == 0.0 {
            // Usar directamente los objetos base si no hay rotación
            base_objects.to_vec()
        } else {
            // Crear objetos rotados solo cuando es necesario
            create_rotated_objects(&base_objects, scene_rotation_angle)
        };
        
        let camera_was_changed = camera.is_changed();
        
        // ========== CONTROLES OPTIMIZADOS ==========
        
        // Detectar modificadores una sola vez
        let shift_pressed = window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || window.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        let zoom_multiplier = if shift_pressed { 3.0 } else { 1.0 };
        
        // ========== CONTROLES DE SKYBOX ==========
        if window.is_key_pressed(KeyboardKey::KEY_ONE) {
            skybox = Skybox::sunset();
        } else if window.is_key_pressed(KeyboardKey::KEY_TWO) {
            skybox = Skybox::midday();
        } else if window.is_key_pressed(KeyboardKey::KEY_THREE) {
            skybox = Skybox::night();
        } else if window.is_key_pressed(KeyboardKey::KEY_FOUR) {
            skybox = Skybox::overcast();
        } else if window.is_key_pressed(KeyboardKey::KEY_FIVE) {
            skybox = Skybox::cosmic();
        }
        
        // ========== CONTROLES DE CÁMARA OPTIMIZADOS ==========
        
        // Órbita de cámara (flechas)
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
        
        // Zoom optimizado (W/S con modificador Shift)
        let effective_zoom_speed = zoom_speed * zoom_multiplier;
        if window.is_key_down(KeyboardKey::KEY_W) {
            camera.zoom_in(effective_zoom_speed);
        }
        if window.is_key_down(KeyboardKey::KEY_S) {
            camera.zoom_out(effective_zoom_speed);
        }
        
        // Control de velocidad de zoom
        if window.is_key_pressed(KeyboardKey::KEY_PAGE_UP) && zoom_speed < max_zoom_speed {
            zoom_speed = (zoom_speed + zoom_speed_increment).min(max_zoom_speed);
        }
        if window.is_key_pressed(KeyboardKey::KEY_PAGE_DOWN) && zoom_speed > min_zoom_speed {
            zoom_speed = (zoom_speed - zoom_speed_increment).max(min_zoom_speed);
        }
        
        // ========== CONTROLES DE ROTACIÓN GLOBAL OPTIMIZADOS ==========
        
        // Rotación automática (Q/E)
        if window.is_key_down(KeyboardKey::KEY_Q) {
            scene_rotation_speed = (scene_rotation_speed + rotation_speed_increment).min(max_rotation_speed);
        }
        if window.is_key_down(KeyboardKey::KEY_E) {
            scene_rotation_speed = (scene_rotation_speed - rotation_speed_increment).max(-max_rotation_speed);
        }
        
        // Rotación manual (A/D)
        let manual_rotation_speed = rotation_speed_increment * if shift_pressed { 10.0 } else { 5.0 };
        if window.is_key_down(KeyboardKey::KEY_A) {
            scene_rotation_angle -= manual_rotation_speed;
        }
        if window.is_key_down(KeyboardKey::KEY_D) {
            scene_rotation_angle += manual_rotation_speed;
        }
        
        // Controles especiales (una sola verificación cada uno)
        if window.is_key_pressed(KeyboardKey::KEY_SPACE) {
            scene_rotation_speed = 0.0;
        }
        if window.is_key_pressed(KeyboardKey::KEY_R) {
            scene_rotation_angle = 0.0;
            scene_rotation_speed = 0.0;
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
            render_adaptive(&mut framebuffer, &objects, &camera, &light, &texture_manager, &skybox, current_lod);
        } else if frames_since_camera_change <= 20 {
            // Fase intermedia: renderizado completo si no está hecho
            if !render_complete {
                render(&mut framebuffer, &objects, &camera, &light, &texture_manager, &skybox);
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
                    &skybox,
                    samples_per_frame,
                    &mut current_sample
                );
            }
        }
        
        // Usar el sistema optimizado de blit y caché
        framebuffer.swap_buffers(&mut window, &thread);
    }
}
