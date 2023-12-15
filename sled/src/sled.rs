use crate::{
    color,
    color::{Rgb, Srgb},
    config::{Config, LineSegment},
    error::SledError,
    led::Led,
    Vec2,
};

use std::ops::{Index, IndexMut};
use std::{ops::Range, usize};

#[allow(dead_code)]
pub struct Sled {
    center_point: Vec2,
    leds: Vec<Led>,
    num_leds: usize,
    line_segments: Vec<LineSegment>,
    // utility lookup tables
    line_segment_endpoint_indices: Vec<(usize, usize)>,
    vertex_indices: Vec<usize>,
    index_of_closest: usize,
}

/// Construction, output, and basic sled info.
impl Sled {
    pub fn new(config_file_path: &str) -> Result<Self, SledError> {
        let config = Config::from_toml_file(config_file_path)?;
        let leds_per_segment = Sled::leds_per_segment(&config);
        let leds = Sled::build_led_list(
            &leds_per_segment,
            &config.line_segments,
            &config.center_point,
        );
        let line_segment_endpoint_indices = Sled::line_segment_endpoint_indices(&leds_per_segment);
        let vertex_indices = Sled::vertex_indices(&config);
        let num_leds = leds.len();
        let index_of_closest = leds
            .iter()
            .min_by(|l, r| l.distance().partial_cmp(&r.distance()).unwrap())
            .unwrap()
            .index();

        Ok(Sled {
            center_point: config.center_point,
            leds,
            num_leds,
            line_segments: config.line_segments,
            index_of_closest,
            // utility lookup tables
            line_segment_endpoint_indices,
            vertex_indices,
        })
    }

    pub fn read(&self) -> Vec<Led> {
        self.leds.clone()
    }

    pub fn read_colors<T>(&self) -> Vec<Srgb<T>>
    where
        f32: color::stimulus::IntoStimulus<T>,
    {
        self.leds
            .iter()
            .map(|led| led.color.into_format())
            .collect()
    }

    pub fn center_point(&self) -> Vec2 {
        self.center_point
    }

    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    pub fn num_segments(&self) -> usize {
        self.line_segments.len()
    }

    pub fn num_vertices(&self) -> usize {
        self.vertex_indices.len()
    }

    fn leds_per_segment(config: &Config) -> Vec<usize> {
        config
            .line_segments
            .iter()
            .map(|line| line.num_leds())
            .collect()
    }

    fn build_led_list(
        leds_per_segment: &Vec<usize>,
        line_segments: &Vec<LineSegment>,
        center_point: &Vec2,
    ) -> Vec<Led> {
        let mut leds = vec![];
        let default_color = Rgb::new(0.0, 0.0, 0.0);

        for (segment_index, segment_size) in leds_per_segment.iter().enumerate() {
            for i in 0..*segment_size {
                let segment = &line_segments[segment_index];
                let alpha = i as f32 / (segment_size - 1) as f32;
                let pos = segment.start.lerp(segment.end, alpha);
                let led = Led::new(default_color, pos, leds.len(), segment_index, *center_point);

                leds.push(led);
            }
        }
        leds
    }

    fn line_segment_endpoint_indices(leds_per_segment: &Vec<usize>) -> Vec<(usize, usize)> {
        let mut line_segment_endpoint_indices = vec![];
        let mut last_index = 0;
        for num_leds in leds_per_segment {
            line_segment_endpoint_indices.push((last_index, last_index + num_leds));
            last_index += num_leds;
        }

        line_segment_endpoint_indices
    }

    fn vertex_indices(config: &Config) -> Vec<usize> {
        let mut vertex_indices = vec![];

        let mut last_end_point: Option<Vec2> = None;
        let mut last_index = 0;
        for line in &config.line_segments {
            if Some(line.start) != last_end_point {
                vertex_indices.push(last_index);
            }

            let num_leds = line.num_leds();
            vertex_indices.push(last_index + num_leds - 1);

            last_index += num_leds;
            last_end_point = Some(line.end);
        }

        vertex_indices
    }
}

/// Index-based read and write methods.
impl Sled {
    pub fn get(&self, index: usize) -> Option<&Led> {
        self.leds.get(index)
    }

    pub fn modulate<F: Fn(&Led) -> Rgb>(
        &mut self,
        index: usize,
        color_rule: F,
    ) -> Result<(), SledError> {
        if index >= self.num_leds {
            return Err(SledError {
                message: format!("LED at index {} does not exist.", index),
            });
        }

        let led = &mut self.leds[index];
        led.color = color_rule(&led);
        Ok(())
    }

    pub fn set(&mut self, index: usize, color: Rgb) -> Result<(), SledError> {
        if index >= self.num_leds {
            return Err(SledError {
                message: format!("LED at index {} does not exist.", index),
            });
        }

        self.leds[index].color = color;
        Ok(())
    }

    pub fn set_all(&mut self, color: Rgb) {
        for led in &mut self.leds {
            led.color = color;
        }
    }

    pub fn for_each<F: FnMut(&mut Led)>(&mut self, mut func: F) {
        for led in self.leds.iter_mut() {
            func(led);
        }
    }
}

impl Index<usize> for Sled {
    type Output = Led;

    fn index(&self, index: usize) -> &Self::Output {
        &self.leds[index]
    }
}

impl IndexMut<usize> for Sled {
    fn index_mut(&mut self, index: usize) -> &mut Led {
        &mut self.leds[index]
    }
}

impl Index<Range<usize>> for Sled {
    type Output = [Led];

    fn index(&self, index_range: Range<usize>) -> &[Led] {
        &self.leds[index_range]
    }
}

impl IndexMut<Range<usize>> for Sled {
    fn index_mut(&mut self, index_range: Range<usize>) -> &mut [Led] {
        &mut self.leds[index_range]
    }
}

/// Index range-based read and write methods
impl Sled {
    pub fn get_range(&self, index_range: Range<usize>) -> Result<&[Led], SledError> {
        if index_range.end < self.num_leds {
            Ok(&self.leds[index_range])
        } else {
            Err(SledError {
                message: format!("Index range extends beyond size of system."),
            })
        }
    }

    pub fn modulate_range<F: Fn(&Led) -> Rgb>(
        &mut self,
        index_range: Range<usize>,
        color_rule: F,
    ) -> Result<(), SledError> {
        if index_range.end >= self.num_leds {
            return Err(SledError {
                message: format!("Index range extends beyond size of system."),
            });
        }

        for led in &mut self.leds[index_range] {
            led.color = color_rule(led);
        }

        Ok(())
    }

    pub fn set_range(&mut self, index_range: Range<usize>, color: Rgb) -> Result<(), SledError> {
        if index_range.end >= self.num_leds {
            return Err(SledError {
                message: format!("Index range extends beyond size of system."),
            });
        }

        self.leds[index_range]
            .iter_mut()
            .for_each(|led| led.color = color);
        Ok(())
    }

    pub fn for_each_in_range<F: FnMut(&mut Led)>(
        &mut self,
        index_range: Range<usize>,
        mut func: F,
    ) {
        self.leds[index_range].iter_mut().for_each(|led| func(led));
    }
}

/// Segment-based read and write methods.
impl Sled {
    pub fn get_segment(&self, segment_index: usize) -> Option<&[Led]> {
        let (start, end) = *self.line_segment_endpoint_indices.get(segment_index)?;
        Some(&self.leds[start..end])
    }

    pub fn modulate_segment<F: Fn(&Led) -> Rgb>(
        &mut self,
        segment_index: usize,
        color_rule: F,
    ) -> Result<(), SledError> {
        if segment_index >= self.line_segment_endpoint_indices.len() {
            return Err(SledError {
                message: format!("Segment of index {} does not exist.", segment_index),
            });
        }

        let (start, end) = self.line_segment_endpoint_indices[segment_index];
        for led in &mut self.leds[start..end] {
            led.color = color_rule(led);
        }

        Ok(())
    }

    pub fn set_segment(&mut self, segment_index: usize, color: Rgb) -> Result<(), SledError> {
        if segment_index >= self.line_segment_endpoint_indices.len() {
            return Err(SledError {
                message: format!("No line segment of index {} exists.", segment_index),
            });
        }

        let (start, end) = self.line_segment_endpoint_indices[segment_index];
        self.set_range(start..end, color).unwrap();
        Ok(())
    }

    pub fn get_segments(&self, range: Range<usize>) -> Option<&[Led]> {
        if range.start >= self.line_segment_endpoint_indices.len() {
            None
        } else {
            let (start, _) = *self.line_segment_endpoint_indices.get(range.start)?;
            let (_, end) = *self.line_segment_endpoint_indices.get(range.end)?;
            Some(&self.leds[start..end])
        }
    }

    pub fn modulate_segments<F: Fn(&Led) -> Rgb>(
        &mut self,
        range: Range<usize>,
        color_rule: F,
    ) -> Result<(), SledError> {
        if range.start >= self.line_segment_endpoint_indices.len() {
            return Err(SledError {
                message: format!(
                    "Segment index range extends beyond the number of segments in the system."
                ),
            });
        }

        let (start, _) = self.line_segment_endpoint_indices[range.start];
        let (_, end) = self.line_segment_endpoint_indices[range.end];
        for led in &mut self.leds[start..end] {
            led.color = color_rule(led);
        }
        Ok(())
    }

    pub fn set_segments(&mut self, range: Range<usize>, color: Rgb) -> Result<(), SledError> {
        if range.start >= self.line_segment_endpoint_indices.len() {
            return Err(SledError {
                message: format!(
                    "Segment index range extends beyond the number of segments in the system."
                ),
            });
        }

        let (start, _) = self.line_segment_endpoint_indices[range.start];
        let (_, end) = self.line_segment_endpoint_indices[range.end];
        for led in &mut self.leds[start..end] {
            led.color = color;
        }
        Ok(())
    }

    pub fn for_each_in_segment<F: FnMut(&mut Led, f32)>(
        &mut self,
        segment_index: usize,
        mut func: F,
    ) -> Result<(), SledError> {
        if segment_index >= self.line_segment_endpoint_indices.len() {
            return Err(SledError {
                message: format!("No line segment of index {} exists.", segment_index),
            });
        }

        let (start, end) = self.line_segment_endpoint_indices[segment_index];
        let num_leds_f32 = (end - start) as f32;

        for index in start..end {
            let alpha = (index - start) as f32 / num_leds_f32;
            func(&mut self.leds[index], alpha);
        }

        Ok(())
    }
}

/// Vertex-based read and write methods.
impl Sled {
    pub fn get_vertex(&self, vertex_index: usize) -> Option<&Led> {
        if vertex_index >= self.vertex_indices.len() {
            return None;
        }

        Some(&self.leds[vertex_index])
    }

    pub fn modulate_vertex<F: Fn(&Led) -> Rgb>(
        &mut self,
        vertex_index: usize,
        color_rule: F,
    ) -> Result<(), SledError> {
        if vertex_index >= self.vertex_indices.len() {
            return Err(SledError {
                message: format!("Vertex of index {} does not exist.", vertex_index),
            });
        }

        let led = &mut self.leds[vertex_index];
        led.color = color_rule(&led);
        Ok(())
    }

    pub fn set_vertex(&mut self, vertex_index: usize, color: Rgb) -> Result<(), SledError> {
        if vertex_index >= self.vertex_indices.len() {
            return Err(SledError {
                message: format!("Vertex with index {} does not exist.", vertex_index),
            });
        }

        self.leds[self.vertex_indices[vertex_index]].color = color;
        Ok(())
    }

    pub fn get_vertices(&self) -> Vec<&Led> {
        self.vertex_indices.iter().map(|i| &self.leds[*i]).collect()
    }

    pub fn modulate_vertices<F: Fn(&Led) -> Rgb>(&mut self, color_rule: F) {
        for i in &self.vertex_indices {
            let led = &mut self.leds[*i];
            led.color = color_rule(&led);
        }
    }

    pub fn set_vertices(&mut self, color: Rgb) {
        for i in &self.vertex_indices {
            let led = &mut self.leds[*i];
            led.color = color;
        }
    }

    pub fn for_each_vertex<F: FnMut(&mut Led)>(&mut self, mut f: F) {
        for i in &self.vertex_indices {
            f(&mut self.leds[*i])
        }
    }
}

/// directional read and write methods
impl Sled {
    fn alpha_to_index(&self, segment_alpha: f32, segment_index: usize) -> usize {
        let segment = &self.line_segments[segment_index];
        let startpoint_index = self.line_segment_endpoint_indices[segment_index].0;
        let leds_in_segment = segment.num_leds() as f32;

        let target = startpoint_index + (segment_alpha * leds_in_segment).floor() as usize;
        target
    }

    fn raycast_for_index(&self, start: Vec2, dir: Vec2) -> Option<usize> {
        let dist = 100_000.0;
        let end = start + dir * dist;

        let mut intersection: Option<(f32, usize)> = None;
        for (index, segment) in self.line_segments.iter().enumerate() {
            if let Some(t) = segment.intersects_line(start, end) {
                intersection = Some((t, index));
                break;
            }
        }

        let (alpha, segment_index) = intersection?;
        return Some(self.alpha_to_index(alpha, segment_index));
    }

    /* direction setters/getters */

    pub fn get_at_dir(&self, dir: Vec2) -> Option<&Led> {
        self.get_at_dir_from(dir, self.center_point)
    }

    pub fn get_at_dir_from(&self, dir: Vec2, pos: Vec2) -> Option<&Led> {
        let index_of_closest = self.raycast_for_index(pos, dir)?;
        Some(self.get(index_of_closest)?)
    }

    pub fn modulate_at_dir<F: Fn(&Led) -> Rgb>(
        &mut self,
        dir: Vec2,
        color_rule: F,
    ) -> Result<(), SledError> {
        self.modulate_at_dir_from(dir, self.center_point, color_rule)
    }

    pub fn modulate_at_dir_from<F: Fn(&Led) -> Rgb>(
        &mut self,
        dir: Vec2,
        pos: Vec2,
        color_rule: F,
    ) -> Result<(), SledError> {
        match self.raycast_for_index(pos, dir) {
            Some(index) => {
                let led = &mut self.leds[index];
                led.color = color_rule(&led);
                Ok(())
            }
            None => Err(SledError {
                message: format!("No LED in directon: {} from {}", dir, pos),
            }),
        }
    }

    pub fn set_at_dir(&mut self, dir: Vec2, color: Rgb) -> Result<(), SledError> {
        self.set_at_dir_from(dir, self.center_point, color)
    }

    pub fn set_at_dir_from(&mut self, dir: Vec2, pos: Vec2, color: Rgb) -> Result<(), SledError> {
        match self.raycast_for_index(pos, dir) {
            Some(index) => {
                self.leds[index].color = color;
                Ok(())
            }
            None => Err(SledError {
                message: format!("No LED in directon: {} from {}", dir, pos),
            }),
        }
    }

    /* angle setters/getters */

    pub fn get_at_angle(&self, angle: f32) -> Option<&Led> {
        let dir = Vec2::from_angle(angle);
        self.get_at_dir(dir)
    }

    pub fn get_at_angle_from(&self, angle: f32, center_point: Vec2) -> Option<&Led> {
        let dir = Vec2::from_angle(angle);
        self.get_at_dir_from(dir, center_point)
    }

    pub fn modulate_at_angle<F: Fn(&Led) -> Rgb>(
        &mut self,
        angle: f32,
        color_rule: F,
    ) -> Result<(), SledError> {
        self.modulate_at_angle_from(angle, self.center_point, color_rule)
    }

    pub fn modulate_at_angle_from<F: Fn(&Led) -> Rgb>(
        &mut self,
        angle: f32,
        pos: Vec2,
        color_rule: F,
    ) -> Result<(), SledError> {
        let dir = Vec2::from_angle(angle);
        self.modulate_at_dir_from(dir, pos, color_rule)
    }

    pub fn set_at_angle(&mut self, angle: f32, color: Rgb) -> Result<(), SledError> {
        self.set_at_angle_from(angle, self.center_point, color)
    }

    pub fn set_at_angle_from(
        &mut self,
        angle: f32,
        pos: Vec2,
        color: Rgb,
    ) -> Result<(), SledError> {
        let dir = Vec2::from_angle(angle);
        self.set_at_dir_from(dir, pos, color)
    }
}

/// position-based read and write methods
impl Sled {
    /* closest getters/setters */

    pub fn get_index_of_closest_to(&self, pos: Vec2) -> usize {
        // get the closest point on each segment and bundle relevant info,
        // then find the closest of those points
        let (alpha, _dist_sq, segment_index) = self
            .line_segments
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                let (closest, alpha) = segment.closest_to_point(pos);
                let dist_sq = closest.distance_squared(pos);
                (alpha, dist_sq, index)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        self.alpha_to_index(alpha, segment_index)
    }

    pub fn get_closest(&self) -> &Led {
        &self.leds[self.index_of_closest]
    }

    pub fn get_closest_to(&self, pos: Vec2) -> &Led {
        let index_of_closest = self.get_index_of_closest_to(pos);
        &self.leds[index_of_closest]
    }

    pub fn modulate_closest<F: Fn(&Led) -> Rgb>(&mut self, color_rule: F) {
        let led = &mut self.leds[self.index_of_closest];
        led.color = color_rule(led);
    }

    pub fn modulate_closest_to<F: Fn(&Led) -> Rgb>(&mut self, pos: Vec2, color_rule: F) {
        let index_of_closest = self.get_index_of_closest_to(pos);
        let led = &mut self.leds[index_of_closest];
        led.color = color_rule(led);
    }

    pub fn set_closest(&mut self, color: Rgb) {
        self.leds[self.index_of_closest].color = color;
    }

    pub fn set_closest_to(&mut self, pos: Vec2, color: Rgb) {
        let index_of_closest = self.get_index_of_closest_to(pos);
        self.leds[index_of_closest].color = color;
    }

    /* at distance methods */

    fn indices_at_dist(&self, pos: Vec2, dist: f32) -> Vec<usize> {
        let mut all_at_distance: Vec<usize> = vec![];
        for (segment_index, segment) in self.line_segments.iter().enumerate() {
            for alpha in segment.intersects_circle(pos, dist) {
                let index = self.alpha_to_index(alpha, segment_index);
                all_at_distance.push(index);
            }
        }

        all_at_distance
    }

    pub fn get_at_dist(&self, dist: f32) -> Vec<&Led> {
        self.get_at_dist_from(dist, self.center_point)
    }

    pub fn get_at_dist_from(&self, dist: f32, pos: Vec2) -> Vec<&Led> {
        let mut all_at_distance: Vec<&Led> = vec![];

        for (segment_index, segment) in self.line_segments.iter().enumerate() {
            for alpha in segment.intersects_circle(pos, dist) {
                let index = self.alpha_to_index(alpha, segment_index);
                all_at_distance.push(&self.leds[index]);
            }
        }

        all_at_distance
    }

    pub fn modulate_at_dist<F: Fn(&Led) -> Rgb>(
        &mut self,
        dist: f32,
        color_rule: F,
    ) -> Result<(), SledError> {
        self.modulate_at_dist_from(dist, self.center_point, color_rule)
    }

    pub fn modulate_at_dist_from<F: Fn(&Led) -> Rgb>(
        &mut self,
        dist: f32,
        pos: Vec2,
        color_rule: F,
    ) -> Result<(), SledError> {
        let indices: Vec<usize> = self.indices_at_dist(pos, dist);

        if indices.is_empty() {
            return Err(SledError {
                message: format!("No LEDs exist at a distance of {} from {}", dist, pos),
            });
        }

        for i in indices {
            let led = &mut self.leds[i];
            led.color = color_rule(led);
        }

        Ok(())
    }

    pub fn set_at_dist(&mut self, dist: f32, color: Rgb) -> Result<(), SledError> {
        self.set_at_dist_from(dist, self.center_point, color)
    }

    pub fn set_at_dist_from(&mut self, dist: f32, pos: Vec2, color: Rgb) -> Result<(), SledError> {
        let indices: Vec<usize> = self.indices_at_dist(pos, dist);

        if indices.is_empty() {
            return Err(SledError {
                message: format!(
                    "No LEDs exist at a distance of {} from the center point.",
                    dist
                ),
            });
        }

        for index in indices {
            self.leds[index].color = color;
        }

        Ok(())
    }

    /* within distance methods */

    pub fn get_within_dist(&self, dist: f32) -> Vec<&Led> {
        self.get_within_dist_from(dist, self.center_point)
    }

    pub fn get_within_dist_from(&self, dist: f32, pos: Vec2) -> Vec<&Led> {
        let mut all_within_distance: Vec<&Led> = vec![];

        for (segment_index, segment) in self.line_segments.iter().enumerate() {
            let intersections = segment.intersects_solid_circle(pos, dist);
            let first = intersections.get(0);
            let second = intersections.get(1);

            if first.is_some() && second.is_some() {
                let first = self.alpha_to_index(*first.unwrap(), segment_index);
                let second = self.alpha_to_index(*second.unwrap(), segment_index);
                let range = first.min(second)..first.max(second);
                all_within_distance.extend(&self.leds[range]);
            }
        }

        all_within_distance
    }

    pub fn modulate_within_dist<F: Fn(&Led) -> Rgb>(&mut self, dist: f32, color_rule: F) {
        for led in &mut self.leds {
            if led.distance() < dist {
                led.color = color_rule(led);
            }
        }
    }

    pub fn set_within_dist(&mut self, dist: f32, color: Rgb) {
        for led in &mut self.leds {
            if led.distance() < dist {
                led.color = color;
            }
        }
    }

    pub fn modulate_within_dist_from<F: Fn(&Led) -> Rgb>(
        &mut self,
        dist: f32,
        pos: Vec2,
        color_rule: F,
    ) {
        let target_sq = dist.powi(2);

        for led in &mut self.leds {
            if led.position().distance_squared(pos) < target_sq {
                led.color = color_rule(led);
            }
        }
    }

    pub fn set_within_dist_from(&mut self, dist: f32, pos: Vec2, color: Rgb) {
        let target_sq = dist.powi(2);

        for led in &mut self.leds {
            if led.position().distance_squared(pos) < target_sq {
                led.color = color;
            }
        }
    }
}

/// Filters
impl Sled {
    pub fn filter(&self, filter: impl Fn(&Led) -> bool) -> Vec<&Led> {
        return self.leds.iter().filter(|led| filter(led)).collect();
    }

    pub fn filter_mut(&mut self, filter: impl Fn(&Led) -> bool) -> Vec<&mut Led> {
        return self.leds.iter_mut().filter(|led| filter(led)).collect();
    }

    pub fn filter_by_angle(&self, angle_filter: impl Fn(f32) -> bool) -> Vec<&Led> {
        self.filter(|led| angle_filter(led.angle()))
    }

    pub fn filter_by_angle_mut(&mut self, angle_filter: impl Fn(f32) -> bool) -> Vec<&mut Led> {
        self.filter_mut(|led| angle_filter(led.angle()))
    }

    pub fn filter_by_dir(&self, dir_filter: impl Fn(Vec2) -> bool) -> Vec<&Led> {
        self.filter(|led| dir_filter(led.direction()))
    }

    pub fn filter_by_dir_mut(&mut self, dir_filter: impl Fn(Vec2) -> bool) -> Vec<&mut Led> {
        self.filter_mut(|led| dir_filter(led.direction()))
    }

    pub fn filter_by_pos(&self, pos_filter: impl Fn(Vec2) -> bool) -> Vec<&Led> {
        self.filter(|led| pos_filter(led.position()))
    }

    pub fn filter_by_pos_mut(&mut self, pos_filter: impl Fn(Vec2) -> bool) -> Vec<&mut Led> {
        self.filter_mut(|led| pos_filter(led.position()))
    }

    pub fn filter_by_dist(&self, dist_filter: impl Fn(f32) -> bool) -> Vec<&Led> {
        self.filter(|led| dist_filter(led.distance()))
    }

    pub fn filter_by_dist_mut(&mut self, dist_filter: impl Fn(f32) -> bool) -> Vec<&mut Led> {
        self.filter_mut(|led| dist_filter(led.distance()))
    }

    pub fn filter_by_dist_from(&self, pos: Vec2, dist_filter: impl Fn(f32) -> bool) -> Vec<&Led> {
        self.filter(|led| {
            let dist = pos.distance(led.position());
            dist_filter(dist)
        })
    }

    pub fn filter_by_dist_from_mut(
        &mut self,
        pos: Vec2,
        dist_filter: impl Fn(f32) -> bool,
    ) -> Vec<&mut Led> {
        self.filter_mut(|led| {
            let dist = pos.distance(led.position());
            dist_filter(dist)
        })
    }
}

/// Maps
impl Sled {
    pub fn map(&mut self, led_to_color_map: impl Fn(&Led) -> Rgb) {
        // consider parallelizing, adding a map_parallel method, or making parallelization an opt-in compiler feature.
        for led in &mut self.leds {
            led.color = led_to_color_map(led);
        }
    }

    pub fn map_by_index(&mut self, index_to_color_map: impl Fn(usize) -> Rgb) {
        self.map(|led| index_to_color_map(led.index()));
    }

    pub fn map_by_segment(&mut self, segment_index_to_color_map: impl Fn(usize) -> Rgb) {
        self.map(|led| segment_index_to_color_map(led.segment()));
    }

    pub fn map_by_pos(&mut self, pos_to_color_map: impl Fn(Vec2) -> Rgb) {
        self.map(|led| pos_to_color_map(led.position()));
    }

    pub fn map_by_dir(&mut self, dir_to_color_map: impl Fn(Vec2) -> Rgb) {
        self.map(|led| dir_to_color_map(led.direction()));
    }

    pub fn map_by_dir_from(&mut self, point: Vec2, dir_to_color_map: impl Fn(Vec2) -> Rgb) {
        self.map(|led| {
            let dir = (point - led.position()).normalize_or_zero();
            dir_to_color_map(dir)
        });
    }

    pub fn map_by_angle(&mut self, angle_to_color_map: impl Fn(f32) -> Rgb) {
        self.map(|led| angle_to_color_map(led.angle()));
    }

    pub fn map_by_angle_from(&mut self, point: Vec2, angle_to_color_map: impl Fn(f32) -> Rgb) {
        let pos_x = Vec2::new(0.0, 1.0);
        self.map(|led| {
            let mut angle = (point - led.position()).angle_between(pos_x);
            if angle < 0.0 {
                angle = (2.0 * std::f32::consts::PI) + angle;
            }

            angle_to_color_map(angle)
        });
    }

    pub fn map_by_dist(&mut self, dist_to_color_map: impl Fn(f32) -> Rgb) {
        self.map(|led| dist_to_color_map(led.distance()));
    }

    pub fn map_by_dist_from(&mut self, pos: Vec2, dist_to_color_map: impl Fn(f32) -> Rgb) {
        self.map(|led| {
            let dist = pos.distance(led.position());
            dist_to_color_map(dist)
        });
    }
}

pub trait CollectionOfLeds {
    // Some methods that might make sense:
    // - get_closest_to(), get_furthest_from()
    // - filter() for chaining
    // - etc

    // Indices, ranges, and some others might not make sense.

    fn filter(&self, filter: impl Fn(&Led) -> bool) -> Vec<&Led>;
    fn get_closest_to(&self, center_point: Vec2) -> &Led;
}

pub trait CollectionOfLedsMut {
    // A lot of normal set methods probably don't make the most sense here. More likely use cases are:
    // - set_all()
    // - filter_mut() for chaining
    // - for_each()
    // - mapping methods
    // - etc

    fn filter(&self, filter: impl Fn(&Led) -> bool) -> Vec<&Led>;

    fn set_all(&mut self, color: Rgb);

    fn get_closest_to(&self, pos: Vec2) -> &Led;
    fn get_closest_to_mut(&mut self, pos: Vec2) -> &mut Led;
    fn set_closest_to(&mut self, pos: Vec2, color: Rgb);

    fn map(&mut self, led_to_color_map: impl Fn(&Led) -> Rgb);
}

impl CollectionOfLeds for Vec<&Led> {
    fn filter(&self, filter: impl Fn(&Led) -> bool) -> Vec<&Led> {
        let mut copy = self.clone();
        copy.retain(|led| filter(led));
        copy
    }

    fn get_closest_to(&self, pos: Vec2) -> &Led {
        self.iter()
            .min_by(|l, r| {
                let d1 = l.position().distance_squared(pos);
                let d2 = r.position().distance_squared(pos);
                d1.partial_cmp(&d2).unwrap()
            })
            .unwrap()
    }
}

impl CollectionOfLedsMut for Vec<&mut Led> {
    fn filter(&self, filter: impl Fn(&Led) -> bool) -> Vec<&Led> {
        let mut copy: Vec<&Led> = self.iter().map(|led| &**led).collect();
        copy.retain(|led| filter(led));
        copy
    }

    fn get_closest_to(&self, pos: Vec2) -> &Led {
        self.iter()
            .min_by(|l, r| {
                let d1 = l.position().distance_squared(pos);
                let d2 = r.position().distance_squared(pos);
                d1.partial_cmp(&d2).unwrap()
            })
            .unwrap()
    }

    fn get_closest_to_mut(&mut self, pos: Vec2) -> &mut Led {
        self.iter_mut()
            .min_by(|l, r| {
                let d1 = l.position().distance_squared(pos);
                let d2 = r.position().distance_squared(pos);
                d1.partial_cmp(&d2).unwrap()
            })
            .unwrap()
    }

    fn set_closest_to(&mut self, pos: Vec2, color: Rgb) {
        self.iter_mut()
            .min_by(|l, r| {
                let d1 = l.position().distance_squared(pos);
                let d2 = r.position().distance_squared(pos);
                d1.partial_cmp(&d2).unwrap()
            })
            .unwrap()
            .color = color;
    }

    fn set_all(&mut self, color: Rgb) {
        for led in self {
            led.color = color;
        }
    }

    fn map(&mut self, _led_to_color_map: impl Fn(&Led) -> Rgb) {
        todo!()
    }
}
