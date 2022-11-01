use super::*;

pub struct TileToolConfig {
    snap_distance: f32,
    selected_type: String,
}

impl EditorToolConfig for TileToolConfig {
    fn default(assets: &Assets) -> Self {
        Self {
            snap_distance: assets.config.snap_distance,
            selected_type: assets.tiles.keys().min().unwrap().clone(),
        }
    }
}

pub struct TileTool {
    geng: Geng,
    assets: Rc<Assets>,
    points: Vec<Vec2<f32>>,
    // wind_drag: Option<(usize, Vec2<f32>)>,
    config: TileToolConfig,
}

impl TileTool {
    fn find_hovered_tile(&self, cursor: &Cursor, level: &Level) -> Option<usize> {
        'tile_loop: for (index, tile) in level.tiles.iter().enumerate() {
            for i in 0..3 {
                let p1 = tile.vertices[i];
                let p2 = tile.vertices[(i + 1) % 3];
                if Vec2::skew(p2 - p1, cursor.world_pos - p1) < 0.0 {
                    continue 'tile_loop;
                }
            }
            return Some(index);
        }
        None
    }
}

impl EditorTool for TileTool {
    type Config = TileToolConfig;
    fn new(geng: &Geng, assets: &Rc<Assets>, config: Self::Config) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            points: vec![],
            config,
        }
    }
    fn draw(
        &self,
        cursor: &Cursor,
        level: &Level,
        camera: &geng::Camera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        if self.points.is_empty() {
            if let Some(index) = self.find_hovered_tile(cursor, level) {
                let tile = &level.tiles[index];
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Polygon::new(tile.vertices.into(), Rgba::new(0.0, 0.0, 1.0, 0.5)),
                );
            }
        } else {
            for &p in &self.points {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        AABB::point(p).extend_uniform(0.1),
                        Rgba::new(0.0, 1.0, 0.0, 0.5),
                    ),
                );
            }
            match *self.points {
                [p1] => {
                    self.geng.draw_2d(
                        framebuffer,
                        camera,
                        &draw_2d::Segment::new(
                            Segment::new(p1, cursor.snapped_world_pos),
                            0.1,
                            Rgba::new(1.0, 1.0, 1.0, 0.5),
                        ),
                    );
                }
                [p1, p2] => {
                    self.geng.draw_2d(
                        framebuffer,
                        camera,
                        &draw_2d::Polygon::new(
                            vec![p1, p2, cursor.snapped_world_pos],
                            Rgba::new(1.0, 1.0, 1.0, 0.5),
                        ),
                    );
                }
                _ => unreachable!(),
            }
        }
        // if let Some((_, start)) = editor.wind_drag {
        //     self.geng.draw_2d(
        //         framebuffer,
        //         &self.camera,
        //         &draw_2d::Segment::new(
        //             Segment::new(
        //                 start,
        //                 self.camera.screen_to_world(
        //                     self.framebuffer_size,
        //                     self.geng.window().mouse_pos().map(|x| x as f32),
        //                 ),
        //             ),
        //             0.2,
        //             Rgba::new(1.0, 0.0, 0.0, 0.5),
        //         ),
        //     );
        // }
    }
    fn handle_event(&mut self, cursor: &Cursor, event: &geng::Event, level: &mut Level) {
        match event {
            geng::Event::MouseDown {
                button: geng::MouseButton::Left,
                ..
            } => {
                self.points.push(cursor.snapped_world_pos);
                if self.points.len() == 3 {
                    let mut vertices: [Vec2<f32>; 3] =
                        mem::take(&mut self.points).try_into().unwrap();
                    if Vec2::skew(vertices[1] - vertices[0], vertices[2] - vertices[0]) < 0.0 {
                        vertices.reverse();
                    }
                    level.modify().tiles.push(Tile {
                        vertices,
                        flow: Vec2::ZERO,
                        type_name: self.config.selected_type.clone(),
                    });
                }
            }
            geng::Event::MouseDown {
                button: geng::MouseButton::Right,
                ..
            } => {
                if self.points.is_empty() {
                    if let Some(index) = self.find_hovered_tile(cursor, level) {
                        level.modify().tiles.remove(index);
                    }
                } else {
                    self.points.clear();
                }
            }
            geng::Event::KeyDown { key: geng::Key::X } => {
                let mut options: Vec<&String> = self.assets.tiles.keys().collect();
                options.sort();
                let idx = options
                    .iter()
                    .position(|&s| s == &self.config.selected_type)
                    .unwrap_or(0);
                self.config.selected_type = options[(idx + 1) % options.len()].clone();
            }
            // geng::Event::KeyUp { key } => match key {
            //     geng::Key::W => {
            //         if let Some((index, start)) = editor.wind_drag.take() {
            //             let to = self.camera.screen_to_world(
            //                 self.framebuffer_size,
            //                 self.geng.window().mouse_pos().map(|x| x as f32),
            //             );
            //             level_mut!().tiles[index].flow = to - start;
            //         }
            //     }
            //     _ => {}
            // },
            // geng::Key::W => {
            //     if editor.wind_drag.is_none() {
            //         self.editor.as_mut().unwrap().wind_drag =
            //             self.find_hovered_tile(&self.levels.postjam).map(|index| {
            //                 (
            //                     index,
            //                     self.camera.screen_to_world(
            //                         self.framebuffer_size,
            //                         self.geng.window().mouse_pos().map(|x| x as f32),
            //                     ),
            //                 )
            //             });
            //     }
            // }
            _ => {}
        }
    }

    const NAME: &'static str = "Tile";
}
