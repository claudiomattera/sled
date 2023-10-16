use std::{error::Error, fmt};

mod internal;
use glam::Vec2;
use internal::config::{Config, LineSegment};
use palette::Srgb;

#[allow(dead_code)]
pub struct Sled {
    leds: LEDs,
    //canvas: Canvas,
}

pub struct Canvas {}

#[allow(dead_code)]
pub struct LEDs {
    center_point: Vec2,
    leds: Vec<Srgb>,
    line_segments: Vec<LineSegment>,
}

impl Sled {
    pub fn new(config_file_path: &str) -> Result<Self, SledError> {
        let config = Config::from_toml_file(config_file_path)?;
        let leds_per_strip = config
            .line_segments
            .iter()
            .map(|line| line.length() * line.density);

        let total_leds = leds_per_strip.sum::<f32>() as usize;

        let leds = vec![Srgb::new(0.0, 0.0, 0.0); total_leds];
        // 4. create various utility maps to help us out later when we need to track down the specific leds.

        // 5. construct
        Ok(Sled {
            leds: LEDs {
                center_point: config.center_point,
                line_segments: config.line_segments,
                leds,
            },
        })
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
