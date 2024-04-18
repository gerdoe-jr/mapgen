use crate::position::Position;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;

#[derive(RustEmbed)]
#[folder = "configs/"]
pub struct Configs;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct GenerationConfig {
    /// name of the preset
    pub name: String,

    /// this can contain any description of the generation preset
    pub description: Option<String>,

    /// (min, max) values for inner kernel
    pub inner_size_bounds: (usize, usize),

    /// (min, max) values for outer kernel
    pub outer_size_bounds: (usize, usize),

    /// probability for mutating inner radius
    pub inner_rad_mut_prob: f32,

    /// probability for mutating inner size
    pub inner_size_mut_prob: f32,

    /// probability for mutating outer radius
    pub outer_rad_mut_prob: f32,

    /// probability for mutating outer size
    pub outer_size_mut_prob: f32,

    /// probability weighting for random selection from best to worst towards next goal
    pub step_weights: Vec<i32>,

    // ------- TODO: these should go somewhere else -----
    pub waypoints: Vec<Position>,
}

impl GenerationConfig {
    /// stores GenerationConfig in cwd as <name>.json
    pub fn save(&self, path: &str) {
        let mut file = File::create(path).expect("failed to create config file");
        let serialized = serde_json::to_string_pretty(self).expect("failed to serialize config");
        file.write_all(serialized.as_bytes())
            .expect("failed to write to config file");
    }

    pub fn load(path: &str) -> GenerationConfig {
        let serialized_from_file = fs::read_to_string(path).expect("failed to read config file");
        let deserialized: GenerationConfig =
            serde_json::from_str(&serialized_from_file).expect("failed to deserialize config file");

        deserialized
    }

    pub fn get_configs() -> HashMap<String, GenerationConfig> {
        let mut configs = HashMap::new();

        for file_name in Configs::iter() {
            let file = Configs::get(&file_name).unwrap();
            let data = std::str::from_utf8(&file.data).unwrap();
            let config: GenerationConfig = serde_json::from_str(&data).unwrap();
            configs.insert(config.name.clone(), config);
        }

        configs
    }
}

impl Default for GenerationConfig {
    fn default() -> GenerationConfig {
        GenerationConfig {
            name: "default".to_string(),
            description: None,
            inner_size_bounds: (3, 3),
            outer_size_bounds: (1, 5),
            inner_rad_mut_prob: 0.25,
            inner_size_mut_prob: 0.5,
            outer_rad_mut_prob: 0.25,
            outer_size_mut_prob: 0.5,
            waypoints: vec![
                Position::new(250, 250),
                Position::new(250, 150),
                Position::new(50, 150),
                Position::new(50, 50),
                Position::new(250, 50),
            ],
            step_weights: vec![20, 11, 10, 9],
        }
    }
}