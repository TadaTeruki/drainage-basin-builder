use worley_particle::map::{
    grad::{GradDifferenceType, GradStrategy},
    lerp::InterpolationMethod,
    IDWStrategy, ParticleMap,
};

// fn gradient_to_flatness(gradient: f64) -> Option<f64> {
//     let flatness = 1.0 - gradient.abs() / 5.0;
//     if flatness < 0.0 {
//         return None;
//     }
//     Some(flatness.sqrt())
// }

pub struct FlatnessMap {
    pub particle_map: ParticleMap<f64>,
}

impl FlatnessMap {
    pub fn new(
        elevation_map: &ParticleMap<f64>,
        minimum_neighbor_num: usize,
        sea_level: f64,
        gradient_to_flatness: impl Fn(f64) -> Option<f64>,
    ) -> Self {
        let particle_map = build_flatness_map(
            elevation_map,
            minimum_neighbor_num,
            sea_level,
            gradient_to_flatness,
        );
        Self { particle_map }
    }

    pub fn save_to_file(&self, file_path: &str) {
        self.particle_map
            .write_to_file(file_path)
            .expect("Error writing drainage map");
    }

    pub fn load_from_file(file_path: &str) -> Option<Self> {
        let particle_map = ParticleMap::<f64>::read_from_file(file_path).ok()?;
        Some(Self { particle_map })
    }

    pub fn map(&self) -> &ParticleMap<f64> {
        &self.particle_map
    }
}

fn build_flatness_map(
    elevation_map: &ParticleMap<f64>,
    minimum_neighbor_num: usize,
    sea_level: f64,
    gradient_to_flatness: impl Fn(f64) -> Option<f64>,
) -> ParticleMap<f64> {
    let mut flatness_map = elevation_map
        .iter()
        .filter_map(|(particle, elevation)| {
            if *elevation < sea_level {
                return None;
            }
            let (x, y) = particle.site();
            let gradient = elevation_map.get_gradient(
                x,
                y,
                &GradStrategy {
                    delta: elevation_map.params().scale,
                    difference_type: GradDifferenceType::Central,
                    ..Default::default()
                },
                &InterpolationMethod::IDW(IDWStrategy::default_from_params(elevation_map.params())),
            )?;
            let habitability = gradient_to_flatness(gradient.value)?;
            Some((*particle, habitability))
        })
        .collect::<ParticleMap<f64>>();

    if minimum_neighbor_num > 0 {
        flatness_map = flatness_map
            .iter()
            .filter(|(particle, _)| {
                let surrounding_particles = particle.calculate_voronoi().neighbors;
                let count = surrounding_particles
                    .iter()
                    .filter(|neighbor| flatness_map.get(neighbor).is_some())
                    .count();

                count >= minimum_neighbor_num
            })
            .map(|(particle, flatness)| (*particle, *flatness))
            .collect::<ParticleMap<f64>>();
    }

    flatness_map
}

#[cfg(feature = "visualize")]
mod visualization {
    use gtk4::{cairo::Context, prelude::WidgetExt, DrawingArea};
    use vislayers::{geometry::FocusRange, window::Layer};

    use super::FlatnessMap;

    impl Layer for FlatnessMap {
        fn draw(&self, drawing_area: &DrawingArea, ctx: &Context, focus_range: &FocusRange) {
            let area_width = drawing_area.width();
            let area_height = drawing_area.height();

            let rect = focus_range.to_rect(area_width as f64, area_height as f64);

            for (particle, flatness) in self.particle_map.iter() {
                let color = [1.0, 0.5, 0.0];

                ctx.set_source_rgba(color[0], color[1], color[2], *flatness);

                let polygon = particle.calculate_voronoi().polygon;

                ctx.new_path();
                for (i, point) in polygon.iter().enumerate() {
                    let x = rect.map_coord_x(point.0, 0.0, area_width as f64);
                    let y = rect.map_coord_y(point.1, 0.0, area_height as f64);

                    if i == 0 {
                        ctx.move_to(x, y);
                    } else {
                        ctx.line_to(x, y);
                    }
                }

                ctx.fill().expect("Failed to draw place");
            }
        }
    }
}
