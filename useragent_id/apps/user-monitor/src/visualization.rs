use super::*;
use plotters::prelude::*;
use plotters_egui::draw_plotters;
use egui::plot::{Plot, Points, Line};

impl MonitorApp {
    pub fn draw_timeline(&mut self, ui: &mut Ui) {
        let plot = Plot::new("timeline")
            .view_aspect(2.0)
            .include_x(0.0)
            .include_y(0.0);

        plot.show(ui, |plot_ui| {
            // Create timeline points
            let points: PlotPoints = self.session.read()
                .events
                .iter()
                .enumerate()
                .map(|(i, event)| [i as f64, 1.0])
                .collect();

            // Add event line
            plot_ui.line(Line::new(points)
                .color(Color32::BLUE)
                .name("Events"));

            // Add voice annotations
            let voice_points: PlotPoints = self.session.read()
                .voice_annotations
                .iter()
                .enumerate()
                .map(|(i, ann)| [i as f64, 2.0])
                .collect();

            plot_ui.points(Points::new(voice_points)
                .color(Color32::GREEN)
                .name("Voice"));
        });
    }

    pub fn draw_heatmap(&mut self, ui: &mut Ui) {
        let plot = Plot::new("heatmap")
            .view_aspect(1.0);

        plot.show(ui, |plot_ui| {
            // Create heatmap from mouse positions
            let points: PlotPoints = self.heatmap_data
                .iter()
                .map(|(x, y, intensity)| [*x, *y])
                .collect();

            plot_ui.points(Points::new(points)
                .color(Color32::RED)
                .name("Mouse Activity"));
        });
    }

    pub fn draw_pattern_graph(&mut self, ui: &mut Ui) {
        let plot = Plot::new("patterns")
            .view_aspect(2.0);

        plot.show(ui, |plot_ui| {
            // Draw pattern connections
            for (events, coords) in &self.pattern_graph {
                let points: PlotPoints = coords
                    .iter()
                    .map(|(x, y)| [*x, *y])
                    .collect();

                plot_ui.line(Line::new(points)
                    .color(Color32::YELLOW)
                    .name("Pattern"));
            }
        });
    }

    pub fn update_visualizations(&mut self) {
        // Update heatmap data
        if let Some((x, y)) = self.mouse_tracker.current_pos {
            self.heatmap_data.push((x as f64, y as f64, 1.0));
        }

        // Update pattern graph
        if let Some(patterns) = self.session_analytics.read().common_patterns.last() {
            let coords: Vec<(f64, f64)> = patterns.0.iter()
                .enumerate()
                .map(|(i, event)| (i as f64, event.timestamp.timestamp() as f64))
                .collect();
            
            self.pattern_graph.push((patterns.0.clone(), coords));
        }
    }
}
