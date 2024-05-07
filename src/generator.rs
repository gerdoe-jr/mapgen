use std::{f32::consts::SQRT_2, usize};

use std::collections::BTreeMap;
use timing::Timer;

use crate::map::Overwrite;
use crate::{
    config::GenerationConfig,
    debug::DebugLayer,
    kernel::Kernel,
    map::{BlockType, Map},
    position::{Position, ShiftDirection},
    random::{Random, Seed},
    walker::CuteWalker,
};

use dt::dt_bool;
use macroquad::color::colors;
use ndarray::{s, Array2, Ix2};

pub fn is_freeze(block_type: &&BlockType) -> bool {
    **block_type == BlockType::Freeze
}

pub struct Generator {
    pub walker: CuteWalker,
    pub map: Map,
    pub rnd: Random,
    pub debug_layers: BTreeMap<&'static str, DebugLayer>,
}

impl Generator {
    /// derive a initial generator state based on a GenerationConfig
    pub fn new(config: &GenerationConfig, seed: Seed) -> Generator {
        let spawn = Position::new(50, 250);
        let map = Map::new(300, 300, BlockType::Hookable, spawn.clone());
        let init_inner_kernel = Kernel::new(5, 0.0);
        let init_outer_kernel = Kernel::new(7, 0.0);
        let walker = CuteWalker::new(spawn, init_inner_kernel, init_outer_kernel, config);
        let rnd = Random::new(seed, config);

        let debug_layers = BTreeMap::from([
            ("edge_bugs", DebugLayer::new(true, colors::RED, &map)),
            ("corners", DebugLayer::new(true, colors::BLUE, &map)),
            ("corner_ends", DebugLayer::new(true, colors::GREEN, &map)),
        ]);

        Generator {
            walker,
            map,
            rnd,
            debug_layers,
        }
    }

    pub fn step(&mut self, config: &GenerationConfig) -> Result<(), &'static str> {
        // check if walker has reached goal position
        if self.walker.is_goal_reached(&config.waypoint_reached_dist) == Some(true) {
            self.walker.next_waypoint();
        }

        if !self.walker.finished {
            config.validate()?;

            // randomly mutate kernel
            self.walker.mutate_kernel(config, &mut self.rnd);

            // perform one step
            self.walker
                .probabilistic_step(&mut self.map, config, &mut self.rnd)?;

            // handle platforms
            self.walker.check_platform(
                &mut self.map,
                config.platform_distance_bounds.0,
                config.platform_distance_bounds.1,
            )?;
        }

        Ok(())
    }

    /// Post processing step to fix all existing edge-bugs, as certain inner/outer kernel
    /// configurations do not ensure a min. 1-block freeze padding consistently.
    fn fix_edge_bugs(&mut self) -> Result<Array2<bool>, &'static str> {
        let mut edge_bug = Array2::from_elem((self.map.width, self.map.height), false);
        let width = self.map.width;
        let height = self.map.height;

        for x in 0..width {
            for y in 0..height {
                let value = &self.map.grid[[x, y]];
                if *value == BlockType::Empty {
                    for dx in 0..=2 {
                        for dy in 0..=2 {
                            if dx == 1 && dy == 1 {
                                continue;
                            }

                            let neighbor_x = (x + dx)
                                .checked_sub(1)
                                .ok_or("fix edge bug out of bounds")?;
                            let neighbor_y = (y + dy)
                                .checked_sub(1)
                                .ok_or("fix edge bug out of bounds")?;
                            if neighbor_x < width && neighbor_y < height {
                                let neighbor_value = &self.map.grid[[neighbor_x, neighbor_y]];
                                if *neighbor_value == BlockType::Hookable {
                                    edge_bug[[x, y]] = true;
                                    // break;
                                    // TODO: this should be easy to optimize
                                }
                            }
                        }
                    }

                    if edge_bug[[x, y]] {
                        self.map.grid[[x, y]] = BlockType::Freeze;
                    }
                }
            }
        }

        Ok(edge_bug)
    }

    /// Using a distance transform this function will fill up all empty blocks that are too far
    /// from the next solid/non-empty block
    pub fn fill_area(&mut self, max_distance: &f32) -> Array2<f32> {
        let grid = self.map.grid.map(|val| *val != BlockType::Empty);

        // euclidean distance transform
        let distance = dt_bool::<f32>(&grid.into_dyn())
            .into_dimensionality::<Ix2>()
            .unwrap();

        self.map
            .grid
            .zip_mut_with(&distance, |block_type, distance| {
                // only modify empty blocks
                if *block_type != BlockType::Empty {
                    return;
                }

                if *distance > *max_distance + SQRT_2 {
                    *block_type = BlockType::Hookable;
                } else if *distance > *max_distance {
                    *block_type = BlockType::Freeze;
                }
            });

        distance
    }

    // returns a vec of corner candidates and their respective direction to the wall
    pub fn find_corners(&self) -> Result<Vec<(Position, ShiftDirection)>, &'static str> {
        let mut candidates: Vec<(Position, ShiftDirection)> = Vec::new();

        let width = self.map.width;
        let height = self.map.height;

        let window_size = 2; // 2 -> 5x5 windows

        for window_x in window_size..(width - window_size) {
            for window_y in window_size..(height - window_size) {
                let window = &self.map.grid.slice(s![
                    window_x - window_size..=window_x + window_size,
                    window_y - window_size..=window_y + window_size
                ]);

                if window[[2, 2]] != BlockType::Empty {
                    continue;
                }

                let shapes = [
                    // R1
                    (
                        [
                            &window[[2, 3]],
                            &window[[3, 0]],
                            &window[[3, 1]],
                            &window[[3, 2]],
                            &window[[3, 3]],
                        ],
                        ShiftDirection::Right,
                    ),
                    // R2
                    (
                        [
                            &window[[2, 1]],
                            &window[[3, 1]],
                            &window[[3, 2]],
                            &window[[3, 3]],
                            &window[[3, 4]],
                        ],
                        ShiftDirection::Right,
                    ),
                    // L1
                    (
                        [
                            &window[[2, 3]],
                            &window[[1, 0]],
                            &window[[1, 1]],
                            &window[[1, 2]],
                            &window[[1, 3]],
                        ],
                        ShiftDirection::Left,
                    ),
                    // L2
                    (
                        [
                            &window[[2, 1]],
                            &window[[1, 1]],
                            &window[[1, 2]],
                            &window[[1, 3]],
                            &window[[1, 4]],
                        ],
                        ShiftDirection::Left,
                    ),
                    // U1
                    (
                        [
                            &window[[3, 2]],
                            &window[[0, 1]],
                            &window[[1, 1]],
                            &window[[2, 1]],
                            &window[[3, 1]],
                        ],
                        ShiftDirection::Up,
                    ),
                    // U2
                    (
                        [
                            &window[[1, 2]],
                            &window[[1, 1]],
                            &window[[2, 1]],
                            &window[[3, 1]],
                            &window[[4, 1]],
                        ],
                        ShiftDirection::Up,
                    ),
                    // D1
                    (
                        [
                            &window[[3, 2]],
                            &window[[0, 3]],
                            &window[[1, 3]],
                            &window[[2, 3]],
                            &window[[3, 3]],
                        ],
                        ShiftDirection::Down,
                    ),
                    // D2
                    (
                        [
                            &window[[1, 2]],
                            &window[[1, 3]],
                            &window[[2, 3]],
                            &window[[3, 3]],
                            &window[[4, 3]],
                        ],
                        ShiftDirection::Down,
                    ),
                ];

                for (shape, dir) in shapes {
                    if shape.iter().all(is_freeze) {
                        candidates.push((Position::new(window_x, window_y), dir));
                    }
                }
            }
        }

        Ok(candidates)
    }

    /// if a skip has been found, this returns the end position
    pub fn check_corner_skip(
        &self,
        init_pos: &Position,
        shift: &ShiftDirection,
        tunnel_bounds: (usize, usize),
    ) -> Option<(Position, usize)> {
        let mut pos = init_pos.clone();

        let mut skip_length = 0;
        let mut stage = 0;
        while stage != 4 && skip_length < tunnel_bounds.1 {
            // shift into given direction, abort if invalid shift
            if pos.shift_in_direction(shift, &self.map).is_err() {
                return None;
            };
            let curr_block_type = self.map.grid.get(pos.as_index()).unwrap();

            stage = match (stage, curr_block_type) {
                // proceed to / or stay in stage 1 if freeze is found
                (0 | 1, BlockType::Freeze) => 1,

                // proceed to / or stay in stage 2 if hookable is found
                (1 | 2, BlockType::Hookable) => 2,

                // proceed to / or stay in stage 2 if freeze is found
                (2 | 3, BlockType::Freeze) => 3,

                // proceed to final state if (first) empty block is found
                (3, BlockType::Empty) => 4,

                // no match -> invalid sequence, abort!
                _ => return None,
            };

            skip_length += 1;
        }

        if stage == 4 && skip_length > tunnel_bounds.0 {
            Some((pos, skip_length))
        } else {
            None
        }
    }

    pub fn generate_skip(
        &mut self,
        start_pos: &Position,
        end_pos: &Position,
        shift: &ShiftDirection,
    ) {
        let top_left = Position::new(
            usize::min(start_pos.x, end_pos.x),
            usize::min(start_pos.y, end_pos.y),
        );
        let bot_right = Position::new(
            usize::max(start_pos.x, end_pos.x),
            usize::max(start_pos.y, end_pos.y),
        );

        self.map.set_area(
            &top_left,
            &bot_right,
            &BlockType::Empty,
            &Overwrite::ReplaceSolidFreeze,
        );

        match shift {
            ShiftDirection::Left | ShiftDirection::Right => {
                self.map.set_area(
                    &top_left.shifted_by(0, -1).unwrap(),
                    &bot_right.shifted_by(0, -1).unwrap(),
                    &BlockType::Freeze,
                    &Overwrite::ReplaceSolidOnly,
                );
                self.map.set_area(
                    &top_left.shifted_by(0, 1).unwrap(),
                    &bot_right.shifted_by(0, 1).unwrap(),
                    &BlockType::Freeze,
                    &Overwrite::ReplaceSolidOnly,
                );
            }
            ShiftDirection::Up | ShiftDirection::Down => {
                self.map.set_area(
                    &top_left.shifted_by(-1, 0).unwrap(),
                    &bot_right.shifted_by(-1, 0).unwrap(),
                    &BlockType::Freeze,
                    &Overwrite::ReplaceSolidOnly,
                );
                self.map.set_area(
                    &top_left.shifted_by(1, 0).unwrap(),
                    &bot_right.shifted_by(1, 0).unwrap(),
                    &BlockType::Freeze,
                    &Overwrite::ReplaceSolidOnly,
                );
            }
        }
    }

    pub fn get_all_valid_skips(&mut self, length_bounds: (usize, usize), min_spacing_sqr: usize) {
        // get corner candidates
        let corner_candidates = self.find_corners().expect("corner detection failed");

        // get possible skips
        let mut skips: Vec<(Position, Position, ShiftDirection, usize)> = Vec::new();
        for (start_pos, shift) in corner_candidates {
            if let Some((end_pos, length)) =
                self.check_corner_skip(&start_pos, &shift, length_bounds)
            {
                skips.push((start_pos.clone(), end_pos, shift.clone(), length));
            }
        }

        // pick final selection of skips
        skips.sort_unstable_by(|s1, s2| usize::cmp(&s1.3, &s2.3)); // sort by length
        let mut valid_skips = vec![true; skips.len()];
        for skip_index in 0..skips.len() {
            // skip if already invalidated
            if !valid_skips[skip_index] {
                continue;
            }

            // skip is valid -> invalidate all following conflicting skips
            let (start, end, _, _) = &skips[skip_index];
            for other_index in (skip_index + 1)..skips.len() {
                let (other_start, other_end, _, _) = &skips[other_index];

                if start.distance_squared(other_start) < min_spacing_sqr
                    || start.distance_squared(other_end) < min_spacing_sqr
                    || end.distance_squared(other_start) < min_spacing_sqr
                    || end.distance_squared(other_start) < min_spacing_sqr
                {
                    valid_skips[other_index] = false;
                }
            }
        }

        // generate all remaining valid skips
        for skip_index in 0..skips.len() {
            if valid_skips[skip_index] {
                let (start, end, shift, _) = &skips[skip_index];
                self.generate_skip(start, end, shift);
            }
        }
    }

    pub fn print_time(timer: &Timer, message: &str) {
        println!("{}: {:?}", message, timer.elapsed());
    }

    pub fn post_processing(&mut self, config: &GenerationConfig) {
        let timer = Timer::start();

        let edge_bugs = self.fix_edge_bugs().expect("fix edge bugs failed");
        Generator::print_time(&timer, "fix edge bugs");

        self.map
            .generate_room(&self.map.spawn.clone(), 4, 3, Some(&BlockType::Start))
            .expect("start room generation failed");
        self.map
            .generate_room(&self.walker.pos.clone(), 4, 3, Some(&BlockType::Finish))
            .expect("start finish room generation");
        Generator::print_time(&timer, "place rooms");

        self.fill_area(&config.max_distance);
        Generator::print_time(&timer, "place obstacles");

        self.get_all_valid_skips(config.skip_length_bounds, config.skip_min_spacing_sqr);

        // debug layers
        // let corners_grid = &mut self.debug_layers.get_mut("corners").unwrap().grid;
        // for (pos, _) in &corner_candidates {
        //     corners_grid[pos.as_index()] = true;
        // }

        // for (pos, shift) in corner_candidates {
        //     if let Some(end_pos) = self.check_corner_skip(&pos, &shift, (4, 15)) {
        //         *self
        //             .debug_layers
        //             .get_mut("corner_ends")
        //             .unwrap()
        //             .grid
        //             .get_mut(end_pos.as_index())
        //             .unwrap() = true;
        //
        //         self.generate_skip(&pos, &end_pos, &shift);
        //     }
        // }

        // set debug layers
        self.debug_layers.get_mut("edge_bugs").unwrap().grid = edge_bugs;
        Generator::print_time(&timer, "set debug layers");
    }

    /// Generates an entire map with a single function call. This function is used by the CLI.
    /// It is important to keep this function up to date with the editor generation, so that
    /// fixed seed map generations result in the same map.
    pub fn generate_map(
        max_steps: usize,
        seed: &Seed,
        config: &GenerationConfig,
    ) -> Result<Map, &'static str> {
        let mut gen = Generator::new(config, seed.clone());

        for _ in 0..max_steps {
            if gen.walker.finished {
                break;
            }
            gen.step(config)?;
        }

        gen.post_processing(config);

        Ok(gen.map)
    }
}
