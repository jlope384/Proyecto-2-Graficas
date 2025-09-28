use raylib::prelude::*;

#[derive(Clone)]
pub enum SkyboxType {
    Solid(Vector3),                    // Color sólido
    Gradient(Vector3, Vector3),        // Gradiente de dos colores
    AtmosphericSunset,                // Atmósfera con atardecer
    StarryNight,                      // Noche estrellada
    CloudySky,                        // Cielo nublado
    Space,                            // Espacio exterior
}

pub struct Skybox {
    skybox_type: SkyboxType,
    sun_direction: Vector3,
    time_of_day: f32, // 0.0 = medianoche, 0.5 = mediodía, 1.0 = medianoche
}

impl Skybox {
    pub fn new(skybox_type: SkyboxType) -> Self {
        Skybox {
            skybox_type,
            sun_direction: Vector3::new(0.3, 0.8, 0.5).normalized(),
            time_of_day: 0.6, // Tarde
        }
    }

    pub fn with_sun_direction(mut self, direction: Vector3) -> Self {
        self.sun_direction = direction.normalized();
        self
    }

    pub fn with_time_of_day(mut self, time: f32) -> Self {
        self.time_of_day = time.clamp(0.0, 1.0);
        self
    }

    pub fn get_color(&self, ray_direction: &Vector3) -> Vector3 {
        match &self.skybox_type {
            SkyboxType::Solid(color) => *color,
            SkyboxType::Gradient(top_color, bottom_color) => {
                self.gradient_skybox(ray_direction, *top_color, *bottom_color)
            }
            SkyboxType::AtmosphericSunset => self.atmospheric_sunset(ray_direction),
            SkyboxType::StarryNight => self.starry_night(ray_direction),
            SkyboxType::CloudySky => self.cloudy_sky(ray_direction),
            SkyboxType::Space => self.space_skybox(ray_direction),
        }
    }

    // Gradiente simple entre dos colores
    fn gradient_skybox(&self, ray_direction: &Vector3, top_color: Vector3, bottom_color: Vector3) -> Vector3 {
        // Usar Y component para gradiente vertical
        let t = (ray_direction.y + 1.0) * 0.5; // Normalizar de [-1,1] a [0,1]
        bottom_color.lerp(top_color, t)
    }

    // Skybox atmosférico con sol
    fn atmospheric_sunset(&self, ray_direction: &Vector3) -> Vector3 {
        // Colores base del atardecer
        let horizon_color = Vector3::new(1.0, 0.6, 0.3);     // Naranja
        let zenith_color = Vector3::new(0.3, 0.7, 1.0);      // Azul cielo
        let sun_color = Vector3::new(1.0, 0.9, 0.7);         // Amarillo sol
        let ground_color = Vector3::new(0.4, 0.3, 0.5);      // Púrpura suelo

        // Factores de altura
        let height_factor = (ray_direction.y + 1.0) * 0.5;
        
        // Color base basado en altura
        let mut sky_color = if ray_direction.y > 0.0 {
            // Cielo: gradiente de horizonte a zenith
            zenith_color.lerp(horizon_color, (1.0 - height_factor).powf(0.8))
        } else {
            // "Suelo": color más oscuro
            ground_color
        };

        // Efecto del sol
        let sun_dot = ray_direction.dot(self.sun_direction).max(0.0);
        let sun_intensity = sun_dot.powf(32.0); // Sol concentrado
        let sun_glow = sun_dot.powf(4.0);       // Resplandor del sol

        // Agregar sol y resplandor
        sky_color = sky_color + sun_color * sun_intensity * 2.0;
        sky_color = sky_color + horizon_color * sun_glow * 0.3;

        // Efecto atmosférico cerca del horizonte
        let atmosphere_factor = (1.0 - ray_direction.y.abs()).powf(2.0);
        sky_color = sky_color + Vector3::new(1.0, 0.4, 0.2) * atmosphere_factor * 0.1;

        sky_color
    }

    // Noche estrellada
    fn starry_night(&self, ray_direction: &Vector3) -> Vector3 {
        let night_color = Vector3::new(0.02, 0.02, 0.08);    // Azul muy oscuro
        let star_color = Vector3::new(1.0, 1.0, 0.9);        // Blanco estrella
        
        let mut sky_color = night_color;

        // Gradiente nocturno simple
        let height_factor = (ray_direction.y + 1.0) * 0.5;
        sky_color = sky_color * (0.5 + height_factor * 0.5);

        // Generar estrellas usando noise procedural
        let star_density = self.procedural_noise(*ray_direction * 100.0);
        if star_density > 0.98 { // Solo los valores más altos son estrellas
            let star_brightness = (star_density - 0.98) / 0.02;
            sky_color = sky_color + star_color * star_brightness * 0.8;
        }

        // Luna (opcional)
        let moon_direction = Vector3::new(-0.3, 0.7, 0.6).normalized();
        let moon_dot = ray_direction.dot(moon_direction).max(0.0);
        let moon_intensity = moon_dot.powf(128.0);
        if moon_intensity > 0.3 {
            sky_color = sky_color + Vector3::new(0.8, 0.8, 0.9) * moon_intensity * 0.5;
        }

        sky_color
    }

    // Cielo nublado
    fn cloudy_sky(&self, ray_direction: &Vector3) -> Vector3 {
        let sky_color = Vector3::new(0.6, 0.7, 0.9);         // Azul grisáceo
        let cloud_color = Vector3::new(0.9, 0.9, 0.95);      // Blanco nube
        let dark_cloud = Vector3::new(0.4, 0.4, 0.45);       // Nube oscura

        // Color base del cielo
        let height_factor = (ray_direction.y + 1.0) * 0.5;
        let mut final_color = sky_color * (0.7 + height_factor * 0.3);

        // Generar nubes usando múltiples octavas de ruido
        let cloud_noise1 = self.procedural_noise(*ray_direction * 5.0);
        let cloud_noise2 = self.procedural_noise(*ray_direction * 12.0) * 0.5;
        let cloud_noise3 = self.procedural_noise(*ray_direction * 25.0) * 0.25;
        
        let cloud_factor = (cloud_noise1 + cloud_noise2 + cloud_noise3) / 1.75;
        
        if cloud_factor > 0.3 {
            let cloud_strength = ((cloud_factor - 0.3) / 0.7).min(1.0);
            let mixed_cloud_color = if cloud_factor > 0.6 {
                cloud_color.lerp(dark_cloud, (cloud_factor - 0.6) * 2.5)
            } else {
                cloud_color
            };
            final_color = final_color.lerp(mixed_cloud_color, cloud_strength);
        }

        final_color
    }

    // Espacio exterior con nebulosas
    fn space_skybox(&self, ray_direction: &Vector3) -> Vector3 {
        let space_color = Vector3::new(0.01, 0.01, 0.03);    // Negro espacio
        let nebula_color1 = Vector3::new(0.8, 0.2, 0.6);     // Púrpura nebulosa
        let nebula_color2 = Vector3::new(0.2, 0.6, 0.9);     // Azul nebulosa
        let star_color = Vector3::new(1.0, 1.0, 1.0);        // Blanco estrella

        let mut final_color = space_color;

        // Nebulosas usando ruido fractal
        let nebula_noise1 = self.procedural_noise(*ray_direction * 3.0);
        let nebula_noise2 = self.procedural_noise(*ray_direction * 7.0) * 0.5;
        let combined_nebula = nebula_noise1 + nebula_noise2;

        if combined_nebula > 0.3 {
            let nebula_strength = (combined_nebula - 0.3) * 0.4;
            let nebula_color = nebula_color1.lerp(nebula_color2, nebula_noise2);
            final_color = final_color + nebula_color * nebula_strength;
        }

        // Estrellas densas
        let star_noise = self.procedural_noise(*ray_direction * 150.0);
        if star_noise > 0.95 {
            let star_brightness = (star_noise - 0.95) / 0.05;
            final_color = final_color + star_color * star_brightness;
        }

        // Estrellas más pequeñas
        let small_star_noise = self.procedural_noise(*ray_direction * 300.0);
        if small_star_noise > 0.98 {
            final_color = final_color + star_color * 0.3;
        }

        final_color
    }

    // Función de ruido procedural simple (basada en hash)
    fn procedural_noise(&self, p: Vector3) -> f32 {
        let mut hash = ((p.x * 73856093.0) as i32) ^ ((p.y * 19349663.0) as i32) ^ ((p.z * 83492791.0) as i32);
        hash = (hash ^ (hash >> 13)) * 1274126177;
        hash = hash ^ (hash >> 16);
        (hash as f32 / 2147483647.0).abs()
    }
}

// Implementación del trait Lerp para Vector3 (si no existe)
trait Lerp {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for Vector3 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t.clamp(0.0, 1.0)
    }
}

// Presets de skyboxes comunes
impl Skybox {
    pub fn sunset() -> Self {
        Skybox::new(SkyboxType::AtmosphericSunset)
            .with_sun_direction(Vector3::new(0.5, 0.3, 0.8))
            .with_time_of_day(0.8)
    }

    pub fn midday() -> Self {
        Skybox::new(SkyboxType::Gradient(
            Vector3::new(0.3, 0.7, 1.0),  // Azul cielo
            Vector3::new(0.6, 0.8, 1.0),  // Azul claro horizonte
        )).with_sun_direction(Vector3::new(0.0, 1.0, 0.0))
    }

    pub fn night() -> Self {
        Skybox::new(SkyboxType::StarryNight)
            .with_time_of_day(0.0)
    }

    pub fn overcast() -> Self {
        Skybox::new(SkyboxType::CloudySky)
            .with_time_of_day(0.5)
    }

    pub fn cosmic() -> Self {
        Skybox::new(SkyboxType::Space)
    }
}