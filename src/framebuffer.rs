// framebuffer.rs

use raylib::prelude::*;

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub color_buffer: Image,
    background_color: Color,
    current_color: Color,
    // Optimizaciones de blit
    cached_texture: Option<Texture2D>,
    buffer_dirty: bool,
    pixel_data: Vec<u32>, // Buffer de píxeles optimizado para blit
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let color_buffer = Image::gen_image_color(width as i32, height as i32, Color::BLACK);
        let pixel_count = (width * height) as usize;
        Framebuffer {
            width,
            height,
            color_buffer,
            background_color: Color::BLACK,
            current_color: Color::WHITE,
            cached_texture: None,
            buffer_dirty: true,
            pixel_data: vec![0; pixel_count], // Buffer optimizado
        }
    }

    pub fn clear(&mut self) {
        // Blit optimizado: llenado rápido del buffer
        let bg_color_u32 = color_to_u32(self.background_color);
        self.pixel_data.fill(bg_color_u32);
        self.buffer_dirty = true;
        
        // Fallback para color_buffer por compatibilidad
        self.color_buffer = Image::gen_image_color(self.width as i32, self.height as i32, self.background_color);
    }

    pub fn set_pixel(&mut self, x: u32, y: u32) {
        if x < self.width && y < self.height {
            // Blit optimizado: acceso directo al buffer
            let index = (y * self.width + x) as usize;
            self.pixel_data[index] = color_to_u32(self.current_color);
            self.buffer_dirty = true;
            
            // Mantener compatibilidad con color_buffer
            self.color_buffer.draw_pixel(x as i32, y as i32, self.current_color);
        }
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn _render_to_file(&self, file_path: &str) {
        self.color_buffer.export_image(file_path);
    }

    pub fn swap_buffers(
        &mut self,
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread,
    ) {
        // Solo actualizar textura si el buffer cambió (optimización crítica)
        if self.buffer_dirty || self.cached_texture.is_none() {
            // Crear o actualizar textura desde buffer optimizado
            if let Some(old_texture) = self.cached_texture.take() {
                // Liberar textura anterior
                drop(old_texture);
            }
            
            // Crear nueva textura desde color_buffer
            if let Ok(new_texture) = window.load_texture_from_image(raylib_thread, &self.color_buffer) {
                self.cached_texture = Some(new_texture);
            }
            
            self.buffer_dirty = false;
        }
        
        // Blit final: dibujar textura cacheada (muy rápido)
        let mut renderer = window.begin_drawing(raylib_thread);
        if let Some(ref texture) = self.cached_texture {
            renderer.draw_texture(texture, 0, 0, Color::WHITE);
        }
    }
    
    // Método para copiar regiones de buffer (blit optimizado)
    pub fn blit_region(&mut self, src_x: u32, src_y: u32, width: u32, height: u32, dst_x: u32, dst_y: u32) {
        for y in 0..height {
            let src_row = src_y + y;
            let dst_row = dst_y + y;
            
            if src_row >= self.height || dst_row >= self.height {
                continue;
            }
            
            for x in 0..width {
                let src_col = src_x + x;
                let dst_col = dst_x + x;
                
                if src_col >= self.width || dst_col >= self.width {
                    continue;
                }
                
                let src_index = (src_row * self.width + src_col) as usize;
                let dst_index = (dst_row * self.width + dst_col) as usize;
                
                if src_index < self.pixel_data.len() && dst_index < self.pixel_data.len() {
                    self.pixel_data[dst_index] = self.pixel_data[src_index];
                }
            }
        }
        self.buffer_dirty = true;
    }
    
    // Invalidar caché para forzar actualización
    pub fn invalidate_cache(&mut self) {
        self.buffer_dirty = true;
    }
    
    // Método para mezclar colores suavemente (útil para transiciones LOD)
    pub fn blend_pixel(&mut self, x: u32, y: u32, color: Color, alpha: f32) {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            if index < self.pixel_data.len() {
                // Obtener color actual
                let current = _u32_to_color(self.pixel_data[index]);
                
                // Mezclar colores
                let blended = Color {
                    r: ((current.r as f32) * (1.0 - alpha) + (color.r as f32) * alpha) as u8,
                    g: ((current.g as f32) * (1.0 - alpha) + (color.g as f32) * alpha) as u8,
                    b: ((current.b as f32) * (1.0 - alpha) + (color.b as f32) * alpha) as u8,
                    a: ((current.a as f32) * (1.0 - alpha) + (color.a as f32) * alpha) as u8,
                };
                
                // Actualizar buffer optimizado
                self.pixel_data[index] = color_to_u32(blended);
                self.buffer_dirty = true;
                
                // Mantener compatibilidad con color_buffer
                self.color_buffer.draw_pixel(x as i32, y as i32, blended);
            }
        }
    }
}

// Función auxiliar para convertir Color a u32 (optimización)
#[inline]
fn color_to_u32(color: Color) -> u32 {
    ((color.a as u32) << 24) | ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32)
}

// Función auxiliar para convertir u32 a Color
#[inline]
fn _u32_to_color(value: u32) -> Color {
    Color {
        r: ((value >> 16) & 0xFF) as u8,
        g: ((value >> 8) & 0xFF) as u8,
        b: (value & 0xFF) as u8,
        a: ((value >> 24) & 0xFF) as u8,
    }
}
