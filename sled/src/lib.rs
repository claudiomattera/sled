use std::{error::Error, fmt, ops::Range, usize};

mod internal;
use glam::Vec2;
use internal::config::{Config, LineSegment};

pub mod color {
    pub use palette::rgb::Rgb;
    pub use palette::*;
}

use color::{Rgb, Srgb};

#[allow(dead_code)]
pub struct Sled {
    center_point: Vec2,
    leds: Vec<Rgb>,
    line_segments: Vec<LineSegment>,
    line_segment_to_index_map: Vec<Vec<usize>>,
}

impl Sled {
    pub fn new(config_file_path: &str) -> Result<Self, SledError> {
        let config = Config::from_toml_file(config_file_path)?;
        let leds_per_strip = Sled::leds_per_strip(&config);

        let leds = vec![Rgb::new(0.0, 0.0, 0.0); leds_per_strip.iter().sum()];
        let line_segment_to_index_map = Sled::line_segment_to_index_map(leds_per_strip);
        // 5. construct
        Ok(Sled {
            center_point: config.center_point,
            line_segments: config.line_segments,
            leds,
            line_segment_to_index_map,
        })
    }

    fn leds_per_strip(config: &Config) -> Vec<usize> {
        config
            .line_segments
            .iter()
            .map(|line| (line.length() * line.density).round() as usize)
            .collect()
    }

    fn line_segment_to_index_map(leds_per_strip: Vec<usize>) -> Vec<Vec<usize>> {
        let mut line_segment_to_index_map = vec![];
        let mut last_index = 0;
        for num_leds in &leds_per_strip {
            line_segment_to_index_map.push((last_index..(last_index + num_leds)).collect());
            last_index += num_leds;
        }

        return line_segment_to_index_map;
    }
}

impl Sled {
    pub fn read<T>(&self) -> Vec<Srgb<T>>
    where
        f32: color::stimulus::IntoStimulus<T>,
    {
        let mut output = vec![];
        for color in &self.leds {
            output.push(color.into_format());
        }
        output
    }

    pub fn get(&self, index: usize) -> Option<&Rgb> {
        self.leds.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Rgb> {
        self.leds.get_mut(index)
    }

    pub fn get_range(&self, index_range: Range<usize>) -> &[Rgb] {
        &self.leds[index_range]
    }

    pub fn get_range_mut(&mut self, index_range: Range<usize>) -> &mut [Rgb] {
        &mut self.leds[index_range]
    }

    pub fn set(&mut self, index: usize, color: Rgb) -> Result<(), SledError> {
        let led = self.get_mut(index);
        match led {
            Some(rgb) => *rgb = color,
            None => {
                return Err(SledError::new(
                    format!("LED at index {} does not exist.", index).as_str(),
                ))
            }
        }

        Ok(())
    }

    pub fn set_all(&mut self, color: Rgb) {
        for led_color in self.leds.iter_mut() {
            *led_color = color;
        }
    }

    pub fn set_range(&mut self, index_range: Range<usize>, color: Rgb) -> Result<(), SledError> {
        for index in index_range {
            match self.set(index, color) {
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SledError {
    message: String,
}

impl SledError {
    pub fn new(message: &str) -> Self {
        SledError {
            message: message.to_string(),
        }
    }

    pub fn from_error(e: impl Error) -> Self {
        SledError {
            message: e.to_string(),
        }
    }
}

impl fmt::Display for SledError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for SledError {} // seems we can't have both. Might not be the best design; reconsider.
