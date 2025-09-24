use raylib::prelude::Vector3;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::material::Material;

pub struct Cube {
    pub center: Vector3,
    pub size: f32,
    pub material: Material,
}

impl Cube {
    pub fn new(center: Vector3, size: f32, material: Material) -> Self {
        Cube {
            center,
            size,
            material,
        }
    }

    fn get_uv(&self, point: &Vector3, normal: &Vector3) -> (f32, f32) {
        let half_size = self.size / 2.0;
        let local_point = *point - self.center;
        
        // Determine which face we're on based on the normal
        if normal.x.abs() > 0.9 {
            // Left or right face
            let u = (local_point.z + half_size) / self.size;
            let v = (local_point.y + half_size) / self.size;
            (u, 1.0 - v)
        } else if normal.y.abs() > 0.9 {
            // Top or bottom face
            let u = (local_point.x + half_size) / self.size;
            let v = (local_point.z + half_size) / self.size;
            (u, 1.0 - v)
        } else {
            // Front or back face
            let u = (local_point.x + half_size) / self.size;
            let v = (local_point.y + half_size) / self.size;
            (u, 1.0 - v)
        }
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect {
        let half_size = self.size / 2.0;
        let min = self.center - Vector3::new(half_size, half_size, half_size);
        let max = self.center + Vector3::new(half_size, half_size, half_size);

        // Calculate intersections with all six planes
        let inv_dir = Vector3::new(
            if ray_direction.x.abs() < 1e-6 { 1e6 } else { 1.0 / ray_direction.x },
            if ray_direction.y.abs() < 1e-6 { 1e6 } else { 1.0 / ray_direction.y },
            if ray_direction.z.abs() < 1e-6 { 1e6 } else { 1.0 / ray_direction.z },
        );

        let t1 = (min.x - ray_origin.x) * inv_dir.x;
        let t2 = (max.x - ray_origin.x) * inv_dir.x;
        let t3 = (min.y - ray_origin.y) * inv_dir.y;
        let t4 = (max.y - ray_origin.y) * inv_dir.y;
        let t5 = (min.z - ray_origin.z) * inv_dir.z;
        let t6 = (max.z - ray_origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        // If tmax < 0, ray is intersecting AABB, but the whole AABB is behind us
        if tmax < 0.0 {
            return Intersect::empty();
        }

        // If tmin > tmax, ray doesn't intersect AABB
        if tmin > tmax {
            return Intersect::empty();
        }

        let t = if tmin > 0.0 { tmin } else { tmax };
        
        if t <= 0.0 {
            return Intersect::empty();
        }

        let point = *ray_origin + *ray_direction * t;
        
        // Calculate normal based on which face was hit
        let local_point = point - self.center;
        let normal = if (local_point.x).abs() > (local_point.y).abs() && (local_point.x).abs() > (local_point.z).abs() {
            Vector3::new(local_point.x.signum(), 0.0, 0.0)
        } else if (local_point.y).abs() > (local_point.z).abs() {
            Vector3::new(0.0, local_point.y.signum(), 0.0)
        } else {
            Vector3::new(0.0, 0.0, local_point.z.signum())
        };

        let (u, v) = self.get_uv(&point, &normal);

        Intersect::new(point, normal, t, self.material.clone(), u, v)
    }
}