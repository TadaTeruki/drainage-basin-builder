use std::collections::HashMap;
use worley_particle::{map::ParticleMap, Particle};

use crate::drainage::node::Stream;

use super::node::{DrainageBasinInput, DrainageBasinNode};

pub struct DrainageMap {
    particle_map: ParticleMap<DrainageBasinNode>,
    river_strength: f64,
    river_ignoreable_width_strength: f64,
}

impl DrainageMap {
    pub fn new(
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

    pub fn map(&self) -> &ParticleMap<DrainageBasinNode> {
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
    ) -> Option<Self> {
        let particle_map = ParticleMap::<DrainageBasinNode>::read_from_file(file_path).ok()?;

        Some(Self {
            particle_map,
            river_strength,
            river_ignoreable_width_strength,
        })
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

fn build_drainage_basin(
    terrain_map: &ParticleMap<DrainageBasinInput>,
) -> ParticleMap<DrainageBasinNode> {
    #[derive(Debug, Clone, PartialEq)]
    struct InternalNode {
        area: f64,
        flow_to: Particle,
        slope: f64,
    }

    let nodes = terrain_map
        .iter()
        .map(|(&particle, input)| {
            let voronoi = particle.calculate_voronoi();
            let area = voronoi.area();
            let mut flow_to = None;
            let mut steepest_slope = 0.0;
            let site = particle.site();
            for neighbor in voronoi.neighbors {
                if let Some(neighbor_input) = terrain_map.get(&neighbor) {
                    if neighbor_input.elevation > input.elevation {
                        continue;
                    }
                    let neighbor_site = neighbor.site();
                    let distance = (site.0 - neighbor_site.0).hypot(site.1 - neighbor_site.1);
                    let slope = (neighbor_input.elevation - input.elevation) / distance;
                    if flow_to.is_some() {
                        if slope > steepest_slope {
                            steepest_slope = slope;
                            flow_to = Some(neighbor);
                        }
                    } else {
                        steepest_slope = slope;
                        flow_to = Some(neighbor);
                    }
                }
            }
            if let Some(flow_to) = flow_to {
                (
                    particle,
                    InternalNode {
                        area,
                        flow_to,
                        slope: steepest_slope,
                    },
                )
            } else {
                (
                    particle,
                    InternalNode {
                        area,
                        flow_to: particle,
                        slope: 0.0,
                    },
                )
            }
        })
        .collect::<ParticleMap<InternalNode>>();

    let mut parent_num = HashMap::new();

    for (_, node) in nodes.iter() {
        let flow_to: Particle = node.flow_to;
        parent_num
            .entry(flow_to)
            .and_modify(|e| *e += 1)
            .or_insert(1);
    }

    let mut drainage_area = HashMap::new();

    for (origin_particle, _) in nodes.iter() {
        let mut current = *origin_particle;
        if parent_num.contains_key(&current) {
            continue;
        }
        loop {
            let current_area = nodes.get(&current).unwrap().area;
            drainage_area
                .entry(current)
                .and_modify(|e| {
                    *e += current_area;
                })
                .or_insert(current_area);

            let flow_to = nodes.get(&current).unwrap().flow_to;

            if flow_to == current {
                break;
            }

            let current_drainage_area = *drainage_area.get(&current).unwrap();

            drainage_area
                .entry(flow_to)
                .and_modify(|e| {
                    *e += current_drainage_area;
                })
                .or_insert(current_drainage_area);

            if *parent_num.get(&flow_to).unwrap() > 1 {
                parent_num.entry(flow_to).and_modify(|e| *e -= 1);
                break;
            }

            current = flow_to;
        }
    }

    let mut river_paths = HashMap::new();

    for (particle, node) in nodes.iter() {
        let flow_to = node.flow_to;
        if flow_to == *particle {
            continue;
        }
        let second_flow_to = nodes.get(&flow_to).unwrap().flow_to;
        let (site_0, site_1, site_2) = (particle.site(), flow_to.site(), second_flow_to.site());
        river_paths.insert(*particle, Stream::new(site_0, site_1, site_2));
    }

    nodes
        .iter()
        .filter_map(|(particle, node)| {
            Some((
                *particle,
                DrainageBasinNode {
                    particle: *particle,
                    area: node.area,
                    drainage_area: *drainage_area.get(particle)?,
                    flow_to: node.flow_to,
                    slope: node.slope,
                    main_river: river_paths.get(particle)?.clone(),
                },
            ))
        })
        .collect::<ParticleMap<DrainageBasinNode>>()
}

#[cfg(feature = "visualize")]
mod visualization {
    use gtk4::{cairo::Context, prelude::WidgetExt, DrawingArea};
    use vislayers::{geometry::FocusRange, window::Layer};

    use super::DrainageMap;

    impl Layer for DrainageMap {
        fn draw(&self, drawing_area: &DrawingArea, cr: &Context, focus_range: &FocusRange) {
            let area_width = drawing_area.width();
            let area_height = drawing_area.height();

            let rect = focus_range.to_rect(area_width as f64, area_height as f64);

            if focus_range.radius() > 0.1 {
                for (_, node) in self.map().iter() {
                    let river_width = node.river_width(self.river_strength());
                    if river_width < self.river_ignoreable_width() {
                        continue;
                    }
                    let iter_num = (0.1 / focus_range.radius()).ceil() as usize;

                    let point_0 = node.main_river.evaluate(0.0);
                    let x0 = rect.map_coord_x(point_0.0, 0.0, area_width as f64);
                    let y0 = rect.map_coord_y(point_0.1, 0.0, area_height as f64);

                    cr.move_to(x0, y0);

                    for i in 1..(iter_num + 1) {
                        let t = i as f64 / iter_num as f64;

                        let point_1 = node.main_river.evaluate(t);
                        let x1 = rect.map_coord_x(point_1.0, 0.0, area_width as f64);
                        let y1 = rect.map_coord_y(point_1.1, 0.0, area_height as f64);

                        cr.line_to(x1, y1);
                    }

                    cr.set_line_width(
                        river_width / focus_range.radius() / self.map().params().scale,
                    );
                    cr.set_source_rgb(0.0, 0.0, 1.0);
                    cr.set_line_cap(gtk4::cairo::LineCap::Round);
                    cr.stroke().expect("Failed to draw edge");
                }
            } else {
                let img_width = drawing_area.width();
                let img_height = drawing_area.height();

                for iy in (0..img_height).step_by(6) {
                    for ix in (0..img_width).step_by(6) {
                        let prop_x = (ix as f64) / img_width as f64;
                        let prop_y = (iy as f64) / img_height as f64;

                        let x = rect.min_x + prop_x * rect.width();
                        let y = rect.min_y + prop_y * rect.height();

                        if self.collides_with_river(x, y) {
                            cr.set_source_rgb(0.0, 0.0, 1.0);
                            cr.rectangle(ix as f64 - 1.0, iy as f64 - 1.0, 3.0, 3.0);
                            cr.fill().expect("Failed to fill rectangle");
                        }
                    }
                }
            }
        }
    }
}
