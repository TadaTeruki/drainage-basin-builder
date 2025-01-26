use worley_particle::{map::ParticleMap, Particle};

use crate::{build_drainage_basin, DrainageBasinInput, DrainageBasinNode};

pub struct DrainageMap {
    particle_map: ParticleMap<DrainageBasinNode>,
    river_strength: f64,
    river_ignoreable_width_strength: f64,
}

impl DrainageMap {
    pub fn from_elevation_map(
        elevation_map: &ParticleMap<f64>,
        river_strength: f64,
        river_ignoreable_width_strength: f64,
    ) -> Self {
        let particle_map_input = elevation_map
            .iter()
            .map(|(particle, elevation)| {
                (
                    *particle,
                    DrainageBasinInput {
                        elevation: *elevation,
                    },
                )
            })
            .collect::<ParticleMap<DrainageBasinInput>>();

        let particle_map = build_drainage_basin(&particle_map_input);

        Self {
            particle_map,
            river_strength,
            river_ignoreable_width_strength,
        }
    }

    pub fn particle_map(&self) -> &ParticleMap<DrainageBasinNode> {
        &self.particle_map
    }

    pub fn river_strength(&self) -> f64 {
        self.river_strength
    }

    pub fn river_ignoreable_width(&self) -> f64 {
        self.river_ignoreable_width_strength * self.particle_map.params().scale
    }

    pub fn save_to_file(&self, file_path: &str) {
        self.particle_map
            .write_to_file(file_path)
            .expect("Error writing drainage map");
    }

    pub fn load_from_file(
        file_path: &str,
        river_strength: f64,
        river_ignoreable_width_strength: f64,
    ) -> Self {
        let particle_map = ParticleMap::<DrainageBasinNode>::read_from_file(file_path)
            .expect("Error reading drainage map");

        Self {
            particle_map,
            river_strength,
            river_ignoreable_width_strength,
        }
    }

    pub fn collides_with_river(&self, x: f64, y: f64) -> bool {
        let radius = self.particle_map.params().scale * 2.0;
        let binding = Particle::from_inside_radius(x, y, *self.particle_map.params(), radius);
        let particles = binding
            .iter()
            .filter_map(|particle| self.particle_map.get(particle));
        for node in particles {
            let river_width = node.river_width(self.river_strength);
            if river_width < self.river_ignoreable_width() {
                continue;
            }

            if node.main_river.collides(x, y, river_width) {
                return true;
            }
        }
        false
    }
}
