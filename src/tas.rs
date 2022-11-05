use geng::prelude::*;

/// A wrapper for a game that implements TAS functionality:
/// save states, slow motion, input replay.
pub struct Tas<T: Tasable> {
    geng: Geng,
    /// The game state that is manipulated.
    game: T,
    show_ui: bool,
    /// Multiplier for `delta_time`, used for slow-motion.
    time_scale: f64,
    /// The expected time between fixed updates.
    fixed_delta_time: f64,
    /// The time until next the fixed update (if queued).
    next_fixed_update: Option<f64>,
    /// All saved states.
    saved_states: Vec<T::Saved>,
}

/// Holds the implementation details of the game to be TAS'ed.
pub trait Tasable {
    /// A type used for saving and restoring the state of the game.
    type Saved: Clone;

    /// Save current state.
    fn save(&self) -> Self::Saved;

    /// Restore a previously saved state.
    fn load(&mut self, state: Self::Saved);
}

impl<T: Tasable> Tas<T> {
    pub fn new(game: T, geng: &Geng) -> Self {
        Self {
            geng: geng.clone(),
            game,
            show_ui: true,
            time_scale: 1.0,
            fixed_delta_time: 1.0,
            next_fixed_update: None,
            saved_states: Vec::new(),
        }
    }

    /// Saves the current game state.
    fn save_state(&mut self) {
        self.saved_states.push(self.game.save());
    }

    /// Attempts to load the saved state by index.
    /// If such a state is not found, nothing happens.
    fn load_state(&mut self, index: usize) {
        // Get the state by index
        if let Some(state) = self.saved_states.get(index) {
            self.game.load(state.clone());
        }
    }

    /// Changes the time scale by the given `delta`.
    /// The final time scale is clamped between 0 and 2.
    fn change_time_scale(&mut self, delta: f64) {
        self.set_time_scale(self.time_scale + delta);
    }

    /// Set the time scale to the given `value` clamped between 0 and 2.
    fn set_time_scale(&mut self, value: f64) {
        self.time_scale = value.clamp(0.0, 2.0);
    }

    /// Handle the `geng::KeyDown { key: geng::Key::Num<num> }` event.
    fn num_down(&mut self, mut num: usize) {
        if num == 0 {
            num = 10;
        }
        if self.geng.window().is_key_pressed(geng::Key::L) {
            // Load state
            num -= 1;
            self.load_state(num);
        } else {
            // Set time scale
            self.set_time_scale(num as f64 * 0.1);
        }
    }
}

impl<T: geng::State + Tasable> geng::State for Tas<T> {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.game.draw(framebuffer);
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time * self.time_scale;
        self.game.update(delta_time);

        if let Some(time) = &mut self.next_fixed_update {
            // Simulate fixed updates manually
            *time -= delta_time;
            let mut updates = 0;
            while *time <= 0.0 {
                *time += self.fixed_delta_time;
                updates += 1;
            }
            for _ in 0..updates {
                self.game.fixed_update(self.fixed_delta_time);
            }
        }
    }

    fn fixed_update(&mut self, delta_time: f64) {
        self.fixed_delta_time = delta_time;
        if self.next_fixed_update.is_none() {
            self.next_fixed_update = Some(delta_time);
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        let window = self.geng.window();
        if window.is_key_pressed(geng::Key::LAlt) {
            // Capture the event
            match event {
                geng::Event::Wheel { delta } => {
                    self.change_time_scale(delta * 0.002);
                }
                geng::Event::KeyDown { key } => match key {
                    geng::Key::F1 => self.show_ui = !self.show_ui,
                    geng::Key::Num0 => self.num_down(0),
                    geng::Key::Num1 => self.num_down(1),
                    geng::Key::Num2 => self.num_down(2),
                    geng::Key::Num3 => self.num_down(3),
                    geng::Key::Num4 => self.num_down(4),
                    geng::Key::Num5 => self.num_down(5),
                    geng::Key::Num6 => self.num_down(6),
                    geng::Key::Num7 => self.num_down(7),
                    geng::Key::Num8 => self.num_down(8),
                    geng::Key::Num9 => self.num_down(9),
                    geng::Key::S => self.save_state(),
                    geng::Key::L => self.load_state(self.saved_states.len() - 1),
                    _ => {}
                },
                _ => {}
            }
            return;
        }

        self.game.handle_event(event);
    }

    fn ui<'a>(&'a mut self, ui: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        if !self.show_ui {
            return self.game.ui(ui);
        }

        use geng::ui::{column, *};
        let mut load_state = None;
        let mut delete_state = None;
        let mut saved_states: Vec<_> = self
            .saved_states
            .iter()
            .enumerate()
            .map(|(i, _)| {
                Box::new(
                    row![
                        Text::new(
                            format!("Save #{i}"),
                            self.geng.default_font().clone(),
                            30.0,
                            Rgba::BLACK
                        ),
                        {
                            let load_save = Button::new(ui, "Load");
                            if load_save.was_clicked() {
                                load_state = Some(i);
                            }
                            load_save
                        }
                        .padding_horizontal(20.0),
                        {
                            let delete_save = Button::new(ui, "Delete");
                            if delete_save.was_clicked() {
                                delete_state = Some(i);
                            }
                            delete_save
                        }
                        .padding_horizontal(20.0),
                    ]
                    .padding_vertical(10.0),
                ) as Box<dyn Widget>
            })
            .collect();
        if let Some(i) = delete_state {
            self.saved_states.remove(i);
        } else if let Some(i) = load_state {
            self.load_state(i);
        }

        let tas_ui = stack![
            column![
                {
                    let time_slider = self::Slider::new(ui, self.time_scale, 0.0..=2.0);
                    if let Some(value) = time_slider.get_change() {
                        self.time_scale = value;
                    }
                    time_slider
                },
                Text::new(
                    format!("Time scale: {:.2}", self.time_scale),
                    self.geng.default_font().clone(),
                    50.0,
                    Rgba::BLACK
                )
                .center()
            ]
            .align(vec2(0.5, 1.0)),
            column({
                saved_states.push(Box::new({
                    let new_save = Button::new(ui, "Save state");
                    if new_save.was_clicked() {
                        self.save_state();
                    }
                    new_save
                }) as Box<dyn Widget>);
                saved_states
            })
            .align(vec2(1.0, 0.5))
        ]
        .uniform_padding(30.0);

        Box::new(stack(vec![self.game.ui(ui), Box::new(tas_ui)]))
    }
}

pub struct Slider<'a> {
    cx: &'a geng::ui::Controller,
    sense: &'a mut geng::ui::Sense,
    pos: &'a mut Option<AABB<f64>>,
    tick_radius: &'a mut f32,
    value: f64,
    range: RangeInclusive<f64>,
    change: RefCell<&'a mut Option<f64>>,
}

impl<'a> Slider<'a> {
    const ANIMATION_SPEED: f32 = 5.0;

    pub fn new(cx: &'a geng::ui::Controller, value: f64, range: RangeInclusive<f64>) -> Self {
        Slider {
            cx,
            sense: cx.get_state(),
            tick_radius: cx.get_state(),
            pos: cx.get_state(),
            value,
            range,
            change: RefCell::new(cx.get_state()),
        }
    }

    pub fn get_change(&self) -> Option<f64> {
        self.change.borrow_mut().take()
    }
}

impl<'a> geng::ui::Widget for Slider<'a> {
    fn sense(&mut self) -> Option<&mut geng::ui::Sense> {
        Some(self.sense)
    }
    fn update(&mut self, delta_time: f64) {
        let target_tick_radius = if self.sense.is_hovered() || self.sense.is_captured() {
            1.0 / 2.0
        } else {
            1.0 / 6.0
        };
        *self.tick_radius += (target_tick_radius - *self.tick_radius)
            .clamp_abs(Self::ANIMATION_SPEED * delta_time as f32);
    }
    fn draw(&mut self, cx: &mut geng::ui::DrawContext) {
        *self.pos = Some(cx.position);
        let geng = cx.geng;
        let draw_2d = geng.draw_2d_helper();
        let position = cx.position.map(|x| x as f32);
        let line_width = position.height() / 3.0;
        let value_position = if self.range.end() == self.range.start() {
            *self.tick_radius
        } else {
            *self.tick_radius
                + ((self.value - *self.range.start()) / (*self.range.end() - *self.range.start()))
                    as f32
                    * (position.width() - line_width)
        };
        geng.draw_2d(
            cx.framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::Quad::new(
                AABB::from_corners(
                    position.bottom_left()
                        + vec2(value_position, (position.height() - line_width) / 2.0),
                    position.top_right()
                        - vec2(line_width / 2.0, (position.height() - line_width) / 2.0),
                ),
                cx.theme.usable_color,
            ),
        );
        draw_2d.circle(
            cx.framebuffer,
            &geng::PixelPerfectCamera,
            position.top_right() - vec2(line_width / 2.0, position.height() / 2.0),
            line_width / 2.0,
            cx.theme.usable_color,
        );
        geng.draw_2d(
            cx.framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::Quad::new(
                AABB::from_corners(
                    position.bottom_left()
                        + vec2(line_width / 2.0, (position.height() - line_width) / 2.0),
                    position.bottom_left()
                        + vec2(value_position, (position.height() + line_width) / 2.0),
                ),
                cx.theme.hover_color,
            ),
        );
        geng.draw_2d(
            cx.framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::Ellipse::circle(
                position.bottom_left() + vec2(line_width / 2.0, position.height() / 2.0),
                line_width / 2.0,
                cx.theme.hover_color,
            ),
        );
        draw_2d.circle(
            cx.framebuffer,
            &geng::PixelPerfectCamera,
            position.bottom_left() + vec2(value_position, position.height() / 2.0),
            *self.tick_radius * position.height(),
            cx.theme.hover_color,
        );
    }
    fn handle_event(&mut self, event: &geng::Event) {
        let aabb = match *self.pos {
            Some(pos) => pos,
            None => return,
        };
        if self.sense.is_captured() {
            if let geng::Event::MouseDown { position, .. }
            | geng::Event::MouseMove { position, .. } = &event
            {
                let position = position.x - aabb.x_min;
                let new_value = *self.range.start()
                    + ((position - aabb.height() / 6.0) / (aabb.width() - aabb.height() / 3.0))
                        .clamp(0.0, 1.0)
                        * (*self.range.end() - *self.range.start());
                **self.change.borrow_mut() = Some(new_value);
            }
        }
    }

    fn calc_constraints(
        &mut self,
        _children: &geng::ui::ConstraintsContext,
    ) -> geng::ui::Constraints {
        geng::ui::Constraints {
            min_size: vec2(1.0, 1.0) * self.cx.theme().text_size as f64,
            flex: vec2(1.0, 0.0),
        }
    }
}
