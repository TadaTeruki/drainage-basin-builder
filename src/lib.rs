use std::collections::HashMap;

use worley_particle::{
    map::{ParticleMap, ParticleMapAttributeRW},
    Particle,
};

#[derive(Debug, Clone, PartialEq)]
struct InternalNode {
    area: f64,
    flow_to: Particle,
    slope: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrainageBasinInput {
    pub elevation: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrainageBasinNode {
    pub particle: Particle,
    pub area: f64,
    pub drainage_area: f64,
    pub slope: f64,
    pub flow_to: Particle,
}

impl ParticleMapAttributeRW for DrainageBasinNode {
    fn from_strs(s: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let particle = Particle::from_strs(s)?;
        let flow_to = Particle::from_strs(&s[Particle::len_strs()..])?;
        let area = s[Particle::len_strs() * 2].parse()?;
        let drainage_area = s[Particle::len_strs() * 2 + 1].parse()?;
        let slope = s[Particle::len_strs() * 2 + 2].parse()?;

        Ok(DrainageBasinNode {
            particle,
            area,
            drainage_area,
            slope,
            flow_to,
        })
    }

    fn to_strings(&self) -> Vec<String> {
        let particle = self.particle.to_strings();
        let flow_to = self.flow_to.to_strings();
        let others = vec![
            self.area.to_string(),
            self.drainage_area.to_string(),
            self.slope.to_string(),
        ];

        particle.into_iter().chain(flow_to).chain(others).collect()
    }

    fn len_strs() -> usize {
        Particle::len_strs() + Particle::len_strs() + 3
    }
}

impl DrainageBasinNode {
    pub fn direction(&self) -> f64 {
        let site_0 = self.particle.site();
        let site_1 = self.flow_to.site();
        (site_1.1 - site_0.1).atan2(site_1.0 - site_0.0)
    }

    pub fn river_width(&self, strength: f64) -> f64 {
        self.drainage_area.sqrt() * strength
    }
}

pub fn build_drainage_basin(
    terrain_map: &ParticleMap<DrainageBasinInput>,
) -> ParticleMap<DrainageBasinNode> {
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
        let flow_to = node.flow_to;
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
                },
            ))
        })
        .collect::<ParticleMap<DrainageBasinNode>>()
}
