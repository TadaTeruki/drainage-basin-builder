use std::{collections::HashMap, i32};

use worley_particle::{map::ParticleMap, Particle};

#[derive(Debug, Clone, PartialEq)]
struct InternalNode {
    area: f64,
    flow_to: Particle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    area: f64,
    drainage_area: f64,
    flow_to: Particle,
}

pub fn build_drainage_basin(terrain_map: &ParticleMap<f64>) -> ParticleMap<Node> {
    let nodes = terrain_map
        .iter()
        .map(|(&particle, elevation)| {
            let voronoi = particle.calculate_voronoi();
            let area = voronoi.area();
            let mut flow_to = None;
            let mut steepest_slope = 0.0;
            let site = particle.site();
            for neighbor in voronoi.neighbors {
                if let Some(neighbor_elevation) = terrain_map.get(&neighbor) {
                    if neighbor_elevation > elevation {
                        continue;
                    }
                    let neighbor_site = neighbor.site();
                    let distance = (site.0 - neighbor_site.0).hypot(site.1 - neighbor_site.1);
                    let slope = (elevation - neighbor_elevation) / distance;
                    if let Some(_) = flow_to {
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
                (particle, InternalNode { area, flow_to })
            } else {
                (
                    particle,
                    InternalNode {
                        area,
                        flow_to: particle,
                    },
                )
            }
        })
        .collect::<ParticleMap<InternalNode>>();

    let mut parent_num = HashMap::new();

    for (_, node) in nodes.iter() {
        let flow_to = node.flow_to;
        if !parent_num.contains_key(&flow_to) {
            parent_num.insert(flow_to, 1);
        } else {
            *parent_num.get_mut(&flow_to).unwrap() += 1;
        }
    }

    let mut drainage_area = HashMap::new();

    for (particle, node) in nodes.iter() {
        if parent_num.contains_key(&particle) {
            continue;
        }
        let mut particle = *particle;
        loop {
            drainage_area
                .entry(particle)
                .and_modify(|e| {
                    *e += node.area;
                })
                .or_insert(node.area);

            if node.flow_to == particle {
                break;
            }

            drainage_area
                .entry(node.flow_to)
                .and_modify(|e| {
                    *e += node.area;
                })
                .or_insert(node.area);

            particle = node.flow_to;

            if *parent_num.get(&particle).unwrap_or(&i32::MAX) > 1 {
                parent_num.entry(particle).and_modify(|e| *e -= 1);
                break;
            }
        }
    }

    nodes
        .iter()
        .map(|(particle, node)| {
            (
                *particle,
                Node {
                    area: node.area,
                    drainage_area: *drainage_area.get(particle).unwrap(),
                    flow_to: node.flow_to,
                },
            )
        })
        .collect::<ParticleMap<Node>>()
}
