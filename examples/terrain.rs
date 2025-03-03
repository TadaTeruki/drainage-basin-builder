use std::{cell::RefCell, rc::Rc};

use drainage_basin_builder::map::DrainageMap;
use gtk4::{cairo::Context, prelude::WidgetExt, DrawingArea};
use vislayers::{
    colormap::SimpleColorMap,
    geometry::FocusRange,
    window::{Layer, Visualizer},
};
use worley_particle::map::{Band, IDWStrategy, InterpolationMethod, ParticleMap};

struct TerrainMap {
    particle_map: ParticleMap<f64>,
    bands: Vec<Band>,
}

impl TerrainMap {
    fn new(file_path: &str, sea_level: f64) -> Self {
        let particle_map =
            ParticleMap::<f64>::read_from_file(file_path).expect("Error reading terrain map");
        let num_thresholds = 80;

        let thresholds = (0..num_thresholds)
            .map(|i| i as f64 * 0.9 / (num_thresholds - 1) as f64 + sea_level)
            .collect::<Vec<_>>();

        let bands = particle_map
            .isobands(
                particle_map.corners(),
                300000.0,
                &thresholds,
                &InterpolationMethod::IDW(IDWStrategy::default_from_params(particle_map.params())),
                true,
            )
            .expect("Error generating bands");

        Self {
            particle_map,
            bands,
        }
    }
}

fn bands_step(focus_range: &FocusRange) -> usize {
    (2.0_f64.powi((focus_range.radius() * 8.0).ceil() as i32) as usize).min(16) - 1
}

impl Layer for TerrainMap {
    fn draw(&self, drawing_area: &DrawingArea, cr: &Context, focus_range: &FocusRange) {
        let color_map = SimpleColorMap::new(
            vec![
                [100.0, 150.0, 70.0],
                [60.0, 90.0, 55.0],
                [210.0, 210.0, 210.0],
            ],
            vec![0.0, 0.35, 0.6],
        );

        let area_width = drawing_area.width();
        let area_height = drawing_area.height();

        let rect = focus_range.to_rect(area_width as f64, area_height as f64);

        let bands_step = bands_step(focus_range);

        for threshold in self.bands.iter().step_by(bands_step) {
            cr.new_path();
            for polygon in &threshold.polygons {
                for (i, point) in polygon.iter().enumerate().step_by(bands_step) {
                    let x = rect.map_coord_x(
                        point.0 - self.particle_map.params().scale / 2.0,
                        0.0,
                        area_width as f64,
                    );
                    let y = rect.map_coord_y(
                        point.1 - self.particle_map.params().scale / 2.0,
                        0.0,
                        area_height as f64,
                    );

                    if i == 0 {
                        cr.move_to(x, y);
                    } else {
                        cr.line_to(x, y);
                    }
                }

                cr.close_path();
            }
            let color = color_map.get_color(threshold.threshold);
            cr.set_source_rgb(color[0] / 255.0, color[1] / 255.0, color[2] / 255.0);
            cr.fill().expect("Failed to fill polygon");
        }

        cr.set_source_rgb(1.0, 0.0, 0.0);
        cr.arc(
            rect.map_coord_x(0.0, 0.0, area_width as f64),
            rect.map_coord_y(0.0, 0.0, area_height as f64),
            2.0,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        cr.fill().expect("Failed to draw center point");
    }
}

struct DrainageMapWrapped(DrainageMap);

impl Layer for DrainageMapWrapped {
    fn draw(&self, drawing_area: &DrawingArea, cr: &Context, focus_range: &FocusRange) {
        let area_width = drawing_area.width();
        let area_height = drawing_area.height();

        let rect = focus_range.to_rect(area_width as f64, area_height as f64);

        if focus_range.radius() > 0.1 {
            for (_, node) in self.0.particle_map().iter() {
                let river_width = node.river_width(self.0.river_strength());
                if river_width < self.0.river_ignoreable_width() {
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
                    river_width / focus_range.radius() / self.0.particle_map().params().scale,
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

                    if self.0.collides_with_river(x, y) {
                        cr.set_source_rgb(0.0, 0.0, 1.0);
                        cr.rectangle(ix as f64 - 1.0, iy as f64 - 1.0, 3.0, 3.0);
                        cr.fill().expect("Failed to fill rectangle");
                    }
                }
            }
        }
    }
}

fn main() {
    let particlemap_id = "11008264925851530191";
    let terrain_path = format!("./data/in/{}.particlemap", particlemap_id);
    let terrain_map = TerrainMap::new(&terrain_path, 0.0025);
    let drainage_path = format!("./data/out/drainage-{}.particlemap", particlemap_id);
    let drainage_map = DrainageMap::from_elevation_map(&terrain_map.particle_map, 1.0, 0.01);
    drainage_map.save_to_file(&drainage_path);

    let drainage_map = DrainageMap::load_from_file(&drainage_path, 1.0, 0.01);

    let mut visualizer = Visualizer::new(800, 600);
    visualizer.add_layer(Rc::new(RefCell::new(terrain_map)), 0);
    visualizer.add_layer(Rc::new(RefCell::new(DrainageMapWrapped(drainage_map))), 1);
    visualizer.run();
}
