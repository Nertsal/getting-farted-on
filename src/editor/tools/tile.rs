use super::*;

pub struct TileToolConfig {
    snap_distance: f32,
    selected_type: String,
}

impl EditorToolConfig for TileToolConfig {
    fn default(assets: &AssetsHandle) -> Self {
        Self {
            snap_distance: assets.get().config.snap_distance,
            selected_type: assets.get().tiles.keys().min().unwrap().to_owned(),
        }
    }
}

pub struct TileTool {
    geng: Geng,
    assets: AssetsHandle,
    points: Vec<vec2<f32>>,
    wind_drag: Option<(usize, vec2<f32>)>,
    saved_flow: vec2<f32>,
    config: TileToolConfig,
}

impl TileTool {
    fn find_hovered_tile(
        &self,
        cursor: &Cursor,
        level: &Level,
        selected_layer: usize,
    ) -> Option<usize> {
        'tile_loop: for (index, tile) in level.layers[selected_layer].tiles.iter().enumerate() {
            for i in 0..3 {
                let p1 = tile.vertices[i];
                let p2 = tile.vertices[(i + 1) % 3];
                if vec2::skew(p2 - p1, cursor.world_pos - p1) < 0.0 {
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
    fn new(geng: &Geng, assets: &AssetsHandle, config: Self::Config) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            wind_drag: None,
            points: vec![],
            saved_flow: vec2::ZERO,
            config,
        }
    }
    fn draw(
        &self,
        cursor: &Cursor,
        level: &Level,
        selected_layer: usize,
        camera: &geng::Camera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        if self.points.is_empty() {
            if let Some(index) = self.find_hovered_tile(cursor, level, selected_layer) {
                let tile = &level.layers[selected_layer].tiles[index];
                self.geng.draw2d().draw2d(
                    framebuffer,
                    camera,
                    &draw2d::Polygon::new(tile.vertices.into(), Rgba::new(0.0, 0.0, 1.0, 0.5)),
                );
                if self.wind_drag.is_none() {
                    self.geng.draw2d().draw2d(
                        framebuffer,
                        camera,
                        &draw2d::Segment::new(
                            Segment(cursor.world_pos, cursor.world_pos + tile.flow),
                            0.2,
                            Rgba::new(1.0, 0.0, 0.0, 0.5),
                        ),
                    );
                }
            }
        } else {
            for &p in &self.points {
                self.geng.draw2d().draw2d(
                    framebuffer,
                    camera,
                    &draw2d::Quad::new(
                        Aabb2::point(p).extend_uniform(0.1),
                        Rgba::new(0.0, 1.0, 0.0, 0.5),
                    ),
                );
            }
            match *self.points {
                [p1] => {
                    self.geng.draw2d().draw2d(
                        framebuffer,
                        camera,
                        &draw2d::Segment::new(
                            Segment(p1, cursor.snapped_world_pos),
                            0.1,
                            Rgba::new(1.0, 1.0, 1.0, 0.5),
                        ),
                    );
                }
                [p1, p2] => {
                    self.geng.draw2d().draw2d(
                        framebuffer,
                        camera,
                        &draw2d::Polygon::new(
                            vec![p1, p2, cursor.snapped_world_pos],
                            Rgba::new(1.0, 1.0, 1.0, 0.5),
                        ),
                    );
                }
                _ => unreachable!(),
            }
        }
        if let Some((_, start)) = self.wind_drag {
            self.geng.draw2d().draw2d(
                framebuffer,
                camera,
                &draw2d::Segment::new(
                    Segment(start, cursor.world_pos),
                    0.2,
                    Rgba::new(1.0, 0.0, 0.0, 0.5),
                ),
            );
        }
    }
    fn handle_event(
        &mut self,
        cursor: &Cursor,
        event: &geng::Event,
        level: &mut Level,
        selected_layer: usize,
    ) {
        match event {
            geng::Event::MouseDown {
                button: geng::MouseButton::Left,
                ..
            } => {
                self.points.push(cursor.snapped_world_pos);
                // Check points are not too close
                for i in 0..self.points.len() {
                    for j in 0..i {
                        if (self.points[j] - self.points[i]).len() < self.config.snap_distance {
                            self.points.pop();
                            return;
                        }
                    }
                }
                if self.points.len() == 3 {
                    // Check triangle is not too small
                    for i in 0..3 {
                        let p1 = self.points[i];
                        let p2 = self.points[(i + 1) % 3];
                        let p3 = self.points[(i + 2) % 3];
                        if vec2::skew((p2 - p1).normalize_or_zero(), p3 - p1).abs()
                            < self.config.snap_distance
                        {
                            self.points.pop();
                            return;
                        }
                    }
                    let mut vertices: [vec2<f32>; 3] =
                        mem::take(&mut self.points).try_into().unwrap();
                    if vec2::skew(vertices[1] - vertices[0], vertices[2] - vertices[0]) < 0.0 {
                        vertices.reverse();
                    }
                    level.modify().layers[selected_layer].tiles.push(Tile {
                        vertices,
                        flow: vec2::ZERO,
                        type_name: self.config.selected_type.clone(),
                    });
                }
            }
            geng::Event::MouseDown {
                button: geng::MouseButton::Right,
                ..
            } => {
                if self.points.is_empty() {
                    if let Some(index) = self.find_hovered_tile(cursor, level, selected_layer) {
                        level.modify().layers[selected_layer].tiles.remove(index);
                    }
                } else {
                    self.points.clear();
                }
            }
            geng::Event::KeyDown { key: geng::Key::X } => {
                let assets = self.assets.get();
                let mut options: Vec<&str> = assets.tiles.keys().collect();
                options.sort();
                let idx = options
                    .iter()
                    .position(|&s| s == &self.config.selected_type)
                    .unwrap_or(0);
                self.config.selected_type = options[(idx + 1) % options.len()].to_owned();
            }

            geng::Event::KeyDown { key: geng::Key::W } => {
                if self.geng.window().is_key_pressed(geng::Key::LCtrl) {
                    if let Some(tile) = self.find_hovered_tile(cursor, level, selected_layer) {
                        let tile = &level.layers[selected_layer].tiles[tile];
                        self.saved_flow = tile.flow;
                    }
                } else if self.geng.window().is_key_pressed(geng::Key::LShift) {
                    if let Some(tile) = self.find_hovered_tile(cursor, level, selected_layer) {
                        level.modify().layers[selected_layer].tiles[tile].flow = self.saved_flow;
                    }
                } else if self.wind_drag.is_none() {
                    self.wind_drag = self
                        .find_hovered_tile(cursor, level, selected_layer)
                        .map(|index| (index, cursor.world_pos));
                }
            }
            geng::Event::KeyUp { key: geng::Key::W } => {
                if let Some((index, start)) = self.wind_drag.take() {
                    self.saved_flow = cursor.world_pos - start;
                    level.modify().layers[selected_layer].tiles[index].flow = self.saved_flow;
                }
            }
            _ => {}
        }
    }

    const NAME: &'static str = "Tile";

    fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        use geng::ui::*;

        let assets = self.assets.get();
        let mut options: Vec<&str> = assets.tiles.keys().collect();
        options.sort();
        let options = column(
            options
                .into_iter()
                .map(|name| {
                    let button = Button::new(cx, name);
                    if button.was_clicked() {
                        self.config.selected_type = name.to_owned();
                    }
                    let mut widget: Box<dyn Widget> =
                        Box::new(button.uniform_padding(8.0).align(vec2(0.0, 0.0)));
                    if *name == self.config.selected_type {
                        widget = Box::new(widget.background_color(Rgba::new(0.5, 0.5, 1.0, 0.5)))
                    }
                    widget
                })
                .collect(),
        );
        options.boxed()
    }
}
