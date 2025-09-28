use raylib::prelude::{Color, Vector3};

#[derive(Clone)]
pub struct Material {
    pub diffuse: Vector3,
    pub albedo: [f32; 4],
    pub specular: f32,
    pub refractive_index: f32,
    pub texture_id: Option<String>,
    pub normal_map_id: Option<String>,
}

impl Material {
    pub fn new(
        diffuse: Vector3,
        specular: f32,
        albedo: [f32; 4],
        refractive_index: f32,
        texture_id: Option<String>,
        normal_map_id: Option<String>,
    ) -> Self {
        Material {
            diffuse,
            albedo,
            specular,
            refractive_index,
            texture_id,
            normal_map_id,
        }
    }

    pub fn black() -> Self {
        Material {
            diffuse: Vector3::zero(),
            albedo: [0.0, 0.0, 0.0, 0.0],
            specular: 0.0,
            refractive_index: 0.0,
            texture_id: None,
            normal_map_id: None,
        }
    }

    // Material 'Tierra/Hierba': Verde/marrón natural con baja reflectividad
    pub fn tierra_hierba() -> Self {
        Material {
            diffuse: Vector3::new(0.4, 0.6, 0.2), // Verde hierba con toque marrón
            albedo: [0.8, 0.1, 0.05, 0.0], // Difuso alto, especular bajo, poca reflexión, sin transparencia
            specular: 15.0, // Especular bajo - superficie mate
            refractive_index: 1.0,
            texture_id: Some("assets/grass_dirt.png".to_string()),
            normal_map_id: Some("assets/grass_dirt_normal.png".to_string()),
        }
    }

    // Material 'Piedra de Castillo': Gris robusto con especular medio
    pub fn piedra_castillo() -> Self {
        Material {
            diffuse: Vector3::new(0.5, 0.5, 0.55), // Gris piedra ligeramente azulado
            albedo: [0.7, 0.2, 0.08, 0.0], // Difuso medio, especular medio, poca reflexión
            specular: 35.0, // Especular medio - piedra pulida
            refractive_index: 1.0,
            texture_id: Some("assets/castle_stone.png".to_string()),
            normal_map_id: Some("assets/castle_stone_normal.png".to_string()),
        }
    }

    // Material 'Agua': Azul translúcido con alta reflexión y transparencia
    pub fn agua() -> Self {
        Material {
            diffuse: Vector3::new(0.1, 0.3, 0.8), // Azul claro agua
            albedo: [0.1, 0.1, 0.7, 0.8], // Poco difuso, poco especular, alta reflexión, alta transparencia
            specular: 80.0, // Especular alto - superficie reflectante
            refractive_index: 1.33, // Índice de refracción del agua
            texture_id: Some("assets/water_waves.png".to_string()),
            normal_map_id: Some("assets/water_normal.png".to_string()),
        }
    }

    // Material 'Lava': Naranja/rojo ardiente con emisión térmica
    pub fn lava() -> Self {
        Material {
            diffuse: Vector3::new(1.0, 0.3, 0.1), // Naranja/rojo intenso
            albedo: [0.9, 0.3, 0.05, 0.0], // Alto difuso (emisivo), especular medio, poca reflexión
            specular: 25.0, // Especular medio - superficie fundida
            refractive_index: 1.0,
            texture_id: Some("assets/lava_bubbles.png".to_string()),
            normal_map_id: Some("assets/lava_normal.png".to_string()),
        }
    }

    // Material 'Cristal/Gema': Transparente con alta reflexión y refracción
    pub fn cristal_gema() -> Self {
        Material {
            diffuse: Vector3::new(0.9, 0.9, 1.0), // Blanco puro con tinte azul cristalino
            albedo: [0.05, 0.1, 0.8, 0.95], // Muy poco difuso, poco especular, alta reflexión, muy alta transparencia
            specular: 150.0, // Especular muy alto - superficie perfectamente pulida
            refractive_index: 1.5, // Índice de refracción del vidrio/cristal
            texture_id: None, // No necesita textura compleja, solo color base
            normal_map_id: None,
        }
    }

    // Variantes de cristal con colores vibrantes
    pub fn cristal_esmeralda() -> Self {
        let mut crystal = Self::cristal_gema();
        crystal.diffuse = Vector3::new(0.1, 0.9, 0.3); // Verde esmeralda vibrante
        crystal
    }

    pub fn cristal_rubi() -> Self {
        let mut crystal = Self::cristal_gema();
        crystal.diffuse = Vector3::new(0.9, 0.1, 0.2); // Rojo rubí vibrante
        crystal
    }

    pub fn cristal_zafiro() -> Self {
        let mut crystal = Self::cristal_gema();
        crystal.diffuse = Vector3::new(0.1, 0.3, 0.9); // Azul zafiro vibrante
        crystal
    }

    // Material 'Madera': Troncos de árboles con textura orgánica
    pub fn madera() -> Self {
        Material {
            diffuse: Vector3::new(0.6, 0.4, 0.2), // Marrón madera natural
            albedo: [0.8, 0.15, 0.05, 0.0], // Alto difuso, poco especular, poca reflexión
            specular: 10.0, // Especular muy bajo - superficie mate y rugosa
            refractive_index: 1.0,
            texture_id: None, // Usar color base por ahora
            normal_map_id: None,
        }
    }

    // Material 'Hojas': Follaje verde vibrante
    pub fn hojas() -> Self {
        Material {
            diffuse: Vector3::new(0.2, 0.8, 0.3), // Verde follaje vibrante
            albedo: [0.9, 0.1, 0.0, 0.0], // Muy difuso, casi sin especular ni reflexión
            specular: 5.0, // Especular mínimo - superficie orgánica
            refractive_index: 1.0,
            texture_id: None, // Color base natural
            normal_map_id: None,
        }
    }

    // Material 'Piedra Oscura': Para ruinas y elementos arquitectónicos antiguos
    pub fn piedra_oscura() -> Self {
        Material {
            diffuse: Vector3::new(0.3, 0.3, 0.35), // Gris oscuro desgastado
            albedo: [0.6, 0.3, 0.1, 0.0], // Difuso medio, especular medio, poca reflexión
            specular: 20.0, // Especular bajo-medio - piedra erosionada
            refractive_index: 1.0,
            texture_id: Some("assets/castle_stone.png".to_string()), // Usar textura de castillo
            normal_map_id: Some("assets/castle_stone_normal.png".to_string()),
        }
    }
}

pub fn vector3_to_color(v: Vector3) -> Color {
    Color::new(
        (v.x * 255.0).min(255.0) as u8,
        (v.y * 255.0).min(255.0) as u8,
        (v.z * 255.0).min(255.0) as u8,
        255,
    )
}