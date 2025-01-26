use std::{collections::HashMap, vec};

use bezier_rs::{Bezier, BezierHandles, TValue};
use glam::DVec2;
use worley_particle::{
    map::{ParticleMap, ParticleMapAttributeRW},
    Particle,
};

pub mod map;

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
pub enum Stream {
    Path(Bezier),
    Point((f64, f64)),
}

impl ParticleMapAttributeRW for Stream {
    fn from_strs(s: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        if s[0] == "Path" {
            let start = DVec2 {
                x: s[1].parse::<f64>()?,
                y: s[2].parse::<f64>()?,
            };
            let handle = DVec2 {
                x: s[3].parse::<f64>()?,
                y: s[4].parse::<f64>()?,
            };
            let end = DVec2 {
                x: s[5].parse::<f64>()?,
                y: s[6].parse::<f64>()?,
            };
            Ok(Stream::Path(Bezier::from_quadratic_coordinates(
                start.x, start.y, handle.x, handle.y, end.x, end.y,
            )))
        } else {
            let x = s[1].parse::<f64>()?;
            let y = s[2].parse::<f64>()?;
            Ok(Stream::Point((x, y)))
        }
    }

    fn to_strings(&self) -> Vec<String> {
        match self {
            Stream::Path(path) => {
                let handle = match path.handles {
                    BezierHandles::Quadratic { handle } => handle,
                    _ => unreachable!(),
                };
                vec![
                    "Path".to_string(),
                    path.start.x.to_string(),
                    path.start.y.to_string(),
                    handle.x.to_string(),
                    handle.y.to_string(),
                    path.end.x.to_string(),
                    path.end.y.to_string(),
                ]
            }
            Stream::Point((x, y)) => vec![
                "Point".to_string(),
                x.to_string(),
                y.to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ],
        }
    }

    fn len_strs() -> usize {
        7
    }
}

impl Stream {
    pub fn new(site_0: (f64, f64), site_1: (f64, f64), site_2: (f64, f64)) -> Self {
        let path_start = ((site_0.0 + site_1.0) / 2.0, (site_0.1 + site_1.1) / 2.0);
        if site_0 == site_1 {
            return Stream::Point(site_0);
        }
        let path_end = ((site_1.0 + site_2.0) / 2.0, (site_1.1 + site_2.1) / 2.0);
        let path = Bezier::from_quadratic_coordinates(
            path_start.0,
            path_start.1,
            site_1.0,
            site_1.1,
            path_end.0,
            path_end.1,
        );

        Stream::Path(path)
    }

    pub fn evaluate(&self, t: f64) -> (f64, f64) {
        match self {
            Stream::Path(path) => {
                let point = path.evaluate(TValue::Parametric(t));
                (point.x, point.y)
            }
            Stream::Point((x, y)) => (*x, *y),
        }
    }

    pub fn collides(&self, x: f64, y: f64, width: f64) -> bool {
        match self {
            Stream::Path(path) => {
                let projection = path.project(DVec2 { x, y }, None);
                let projection_point = path.evaluate(TValue::Parametric(projection));
                let squared_distance =
                    (projection_point.x - x).powi(2) + (projection_point.y - y).powi(2);
                squared_distance < width.powi(2)
            }
            Stream::Point((x0, y0)) => {
                let squared_distance = (x0 - x).powi(2) + (y0 - y).powi(2);
                squared_distance < width.powi(2)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrainageBasinNode {
    pub particle: Particle,
    pub area: f64,
    pub drainage_area: f64,
    pub slope: f64,
    pub flow_to: Particle,
    pub main_river: Stream,
}

impl ParticleMapAttributeRW for DrainageBasinNode {
    fn from_strs(s: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let particle = Particle::from_strs(&s[..Particle::len_strs()])?;
        let flow_to = Particle::from_strs(&s[Particle::len_strs()..Particle::len_strs() * 2])?;
        let main_river = Stream::from_strs(
            &s[Particle::len_strs() * 2..Particle::len_strs() * 2 + Stream::len_strs()],
        )?;
        let area = s[Particle::len_strs() * 2 + Stream::len_strs()].parse::<f64>()?;
        let drainage_area = s[Particle::len_strs() * 2 + Stream::len_strs() + 1].parse::<f64>()?;
        let slope = s[Particle::len_strs() * 2 + Stream::len_strs() + 2].parse::<f64>()?;

        Ok(DrainageBasinNode {
            particle,
            area,
            drainage_area,
            slope,
            flow_to,
            main_river,
        })
    }

    fn to_strings(&self) -> Vec<String> {
        let particle = self.particle.to_strings();
        let flow_to = self.flow_to.to_strings();
        let main_river = self.main_river.to_strings();
        let others = vec![
            self.area.to_string(),
            self.drainage_area.to_string(),
            self.slope.to_string(),
        ];

        particle
            .into_iter()
            .chain(flow_to)
            .chain(main_river)
            .chain(others)
            .collect()
    }

    fn len_strs() -> usize {
        Particle::len_strs() + Particle::len_strs() + Stream::len_strs() + 3
    }
}

impl DrainageBasinNode {
    pub fn direction(&self) -> f64 {
        let site_0 = self.particle.site();
        let site_1 = self.flow_to.site();
        (site_1.1 - site_0.1).atan2(site_1.0 - site_0.0)
    }

    pub fn river_width(&self, strength: f64) -> f64 {
        self.drainage_area.sqrt() * strength * self.particle.params().scale
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
