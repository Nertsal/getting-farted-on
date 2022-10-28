use geng::prelude::*;

/// A wrapper for a game that implements TAS functionality:
/// save states, slow motion, input replay.
pub struct Tas<T: Tasable> {
    geng: Geng,
    /// The game state that is manipulated.
    game: T,
    /// Multiplier for `delta_time`, used for slow-motion.
    time_scale: f64,
    /// The expected time between fixed updates.
    fixed_delta_time: f64,
    /// The time until next the fixed update (if queued).
    next_fixed_update: Option<f64>,
    /// All saved states.
    saved_states: Vec<T::Saved>,
}

// Holds the implementation details of the game to be TAS'ed.
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

        // TAS ui
        let camera = geng::Camera2d {
            center: Vec2::ZERO,
            rotation: 0.0,
            fov: 10.0,
        };
        let text = format!("Time scale: {:.2} ", self.time_scale);
        let text = draw_2d::Text::unit(&**self.geng.default_font(), text, Rgba::BLACK)
            .scale_uniform(0.2)
            .align_bounding_box(vec2(0.5, 0.5))
            .translate(vec2(0.0, 4.3));
        self.geng.draw_2d(framebuffer, &camera, &text);
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
                    // Handle numbers
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
                    // Save state
                    geng::Key::S => self.save_state(),
                    _ => {}
                },
                _ => {}
            }
            return;
        }

        self.game.handle_event(event);
    }
}
