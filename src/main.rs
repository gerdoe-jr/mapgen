mod grid_test;
mod map;
mod position;
mod walker;

use std::usize;

use grid_test::*;
use map::*;
use position::*;
use walker::*;

use egui::{epaint::Shadow, Color32, Frame, Label, Margin, Rect};
use macroquad::prelude::*;

const LEVEL_SIZE: usize = 100;

fn window_frame() -> Frame {
    Frame {
        fill: Color32::from_gray(0),
        inner_margin: Margin::same(5.0),
        shadow: Shadow::NONE,
        ..Default::default()
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "egui with macroquad".to_owned(),
        ..Default::default()
    }
}

// TODO: not quite sure where to put this, this doesnt
// have any functionality, so a seperate file feels overkill
#[derive(Debug)]
pub enum ShiftDirection {
    Up,
    Right,
    Down,
    Left,
}

#[macroquad::main(window_conf)]
async fn main() {
    let kernel = Kernel::new(3, 1.0);
    dbg!(kernel);

    let mut canvas: Rect = Rect::EVERYTHING;
    let mut map = Map::new(LEVEL_SIZE, LEVEL_SIZE, BlockType::Empty);
    let mut walker = CuteWalker::new(Position::new(0, 0));

    // setup waypoints
    let goals: Vec<Position> = vec![
        Position::new(5, 5),
        Position::new(95, 5),
        Position::new(95, 95),
        Position::new(5, 95),
        Position::new(50, 50),
    ];
    let mut goals_iter = goals.iter();
    let mut curr_goal = goals_iter.next().unwrap();

    // very important
    walker.cuddle();

    loop {
        clear_background(WHITE);

        // walker logic
        if walker.pos.ne(&curr_goal) {
            let shift = walker.pos.get_greedy_dir(&curr_goal);
            walker
                .shift_pos(shift, &map)
                .expect("Expecting valid shift here");
            map.grid[walker.pos.as_index()] = BlockType::Filled;
        } else if let Some(next_goal) = goals_iter.next() {
            curr_goal = next_goal;
        }

        // define egui
        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label("hello world");
                ui.separator();
            });

            egui::Window::new("DEBUG")
                .frame(window_frame())
                .show(egui_ctx, |ui| {
                    ui.add(Label::new(get_fps().to_string()));
                    ui.add(Label::new(format!("{:?}", walker)));
                });

            // store remaining space for macroquad drawing
            canvas = egui_ctx.available_rect();
        });

        // draw grid
        let display_factor = (f32::min(canvas.width(), canvas.height())) / LEVEL_SIZE as f32; // TODO: assumes square
        draw_grid_blocks(&mut map.grid, display_factor, vec2(0.0, 0.0));

        // draw egui on top of macroquad
        egui_macroquad::draw();

        next_frame().await
    }
}
