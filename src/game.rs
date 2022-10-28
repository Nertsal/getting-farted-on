use super::*;

pub const CONTROLS_LEFT: [geng::Key; 2] = [geng::Key::A, geng::Key::Left];
pub const CONTROLS_RIGHT: [geng::Key; 2] = [geng::Key::D, geng::Key::Right];
pub const CONTROLS_FORCE_FART: [geng::Key; 3] = [geng::Key::W, geng::Key::Up, geng::Key::Space];

pub struct Game {
    pub best_time: Option<f32>,
    pub emotes: Vec<(f32, Id, usize)>,
    pub best_progress: f32,
    pub framebuffer_size: Vec2<f32>,
    pub prev_mouse_pos: Vec2<f64>,
    pub geng: Geng,
    pub config: Config,
    pub assets: Rc<Assets>,
    pub camera: geng::Camera2d,
    pub levels: Levels,
    pub editor: Option<EditorState>,
    pub guys: Collection<Guy>,
    pub my_guy: Option<Id>,
    pub simulation_time: f32,
    pub remote_simulation_times: HashMap<Id, f32>,
    pub remote_updates: HashMap<Id, std::collections::VecDeque<(f32, Guy)>>,
    pub real_time: f32,
    pub noise: noise::OpenSimplex,
    pub opt: Opt,
    pub farticles: Vec<Farticle>,
    pub volume: f32,
    pub client_id: Id,
    pub connection: Connection,
    pub customization: Guy,
    pub ui_controller: ui::Controller,
    pub buttons: Vec<ui::Button<UiMessage>>,
    pub show_customizer: bool,
    pub old_music: geng::SoundEffect,
    pub new_music: geng::SoundEffect,
    pub show_names: bool,
    pub show_leaderboard: bool,
    pub follow: Option<Id>,
}

impl Drop for Game {
    fn drop(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.save_level(&self.levels);
        }
    }
}

impl tas::Tasable for Game {
    type Saved = Option<Guy>;

    fn save(&self) -> Self::Saved {
        self.my_guy.and_then(|id| self.guys.get(&id)).cloned()
    }

    fn load(&mut self, state: Self::Saved) {
        match state {
            Some(mut guy) => {
                if let Some(id) = self.my_guy {
                    guy.id = id;
                    self.guys.insert(guy);
                }
            }
            None => {
                if let Some(id) = &self.my_guy {
                    self.guys.remove(id);
                }
            }
        }
    }
}

impl Game {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        levels: Levels,
        opt: Opt,
        client_id: Id,
        connection: Connection,
    ) -> Self {
        let mut result = Self {
            best_time: None,
            emotes: vec![],
            geng: geng.clone(),
            config: assets.config.clone(),
            assets: assets.clone(),
            camera: geng::Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 5.0,
            },
            framebuffer_size: vec2(1.0, 1.0),
            editor: if opt.editor {
                Some(EditorState::new())
            } else {
                None
            },
            levels,
            guys: Collection::new(),
            my_guy: None,
            real_time: 0.0,
            noise: noise::OpenSimplex::new(),
            prev_mouse_pos: Vec2::ZERO,
            opt: opt.clone(),
            farticles: default(),
            volume: assets.config.volume,
            client_id,
            connection,
            simulation_time: 0.0,
            remote_simulation_times: HashMap::new(),
            remote_updates: default(),
            customization: {
                let mut guy = Guy::new(-1, vec2(0.0, 0.0), false);
                if opt.postjam {
                    guy.postjam = true;
                }
                guy
            },
            best_progress: 0.0,
            ui_controller: ui::Controller::new(geng, assets),
            buttons: vec![
                ui::Button::new("PLAY", vec2(0.0, -3.0), 1.0, 0.5, UiMessage::Play),
                ui::Button::new(
                    "randomize",
                    vec2(2.0, 0.0),
                    0.7,
                    0.0,
                    UiMessage::RandomizeSkin,
                ),
                ui::Button::new(
                    &format!("postjam ({})", if opt.postjam { "on" } else { "off" }),
                    vec2(0.0, -4.0),
                    0.7,
                    0.5,
                    UiMessage::TogglePostJam,
                ),
            ],
            show_customizer: !opt.editor,
            old_music: {
                let mut effect = assets.sfx.old_music.play();
                effect.set_volume(0.0);
                effect
            },
            new_music: {
                let mut effect = assets.sfx.new_music.play();
                effect.set_volume(0.0);
                effect
            },
            show_names: true,
            show_leaderboard: true,
            follow: None,
        };
        if !opt.editor {
            result.my_guy = Some(client_id);
            result.guys.insert(Guy::new(
                client_id,
                result.levels.get(result.customization.postjam).spawn_point,
                true,
            ));
        }
        result
    }

    fn draw_progress(&mut self, framebuffer: &mut ugli::Framebuffer) {
        if self.show_customizer {
            return;
        }
        if let Some(id) = self.my_guy {
            let camera = geng::Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 10.0,
            };
            let guy = self.guys.get_mut(&id).unwrap();
            if guy.finished {
                self.assets.font.draw(
                    framebuffer,
                    &camera,
                    &"GG",
                    vec2(0.0, 3.0),
                    geng::TextAlign::CENTER,
                    1.5,
                    Rgba::BLACK,
                );
            }
            let progress = {
                let level = self.levels.get(self.customization.postjam);
                let mut total_len = 0.0;
                for window in level.expected_path.windows(2) {
                    let a = window[0];
                    let b = window[1];
                    total_len += (b - a).len();
                }
                let mut progress = 0.0;
                let mut closest_point_distance = 1e9;
                let mut prefix_len = 0.0;
                for window in level.expected_path.windows(2) {
                    let a = window[0];
                    let b = window[1];
                    let v = Surface {
                        p1: a,
                        p2: b,
                        type_name: String::new(),
                    }
                    .vector_from(guy.pos);
                    if v.len() < closest_point_distance {
                        closest_point_distance = v.len();
                        progress = (prefix_len + (guy.pos + v - a).len()) / total_len;
                    }
                    prefix_len += (b - a).len();
                }
                progress
            };
            guy.progress = progress;
            self.best_progress = self.best_progress.max(progress);
            guy.best_progress = self.best_progress;
            if guy.finished && self.simulation_time < self.best_time.unwrap_or(1e9) {
                self.best_time = Some(self.simulation_time);
            }
            guy.best_time = self.best_time;
            let mut time_text = String::new();
            let seconds = self.simulation_time.round() as i32;
            let minutes = seconds / 60;
            let seconds = seconds % 60;
            let hours = minutes / 60;
            let minutes = minutes % 60;
            if hours != 0 {
                time_text += &format!("{} hours ", hours);
            }
            if minutes != 0 {
                time_text += &format!("{} minutes ", minutes);
            }
            time_text += &format!("{} seconds", seconds);
            self.assets.font.draw(
                framebuffer,
                &camera,
                &time_text,
                vec2(0.0, -3.3),
                geng::TextAlign::CENTER,
                0.5,
                Rgba::BLACK,
            );
            self.assets.font.draw(
                framebuffer,
                &camera,
                &"progress",
                vec2(0.0, -4.0),
                geng::TextAlign::CENTER,
                0.5,
                Rgba::BLACK,
            );
            self.geng.draw_2d(
                framebuffer,
                &camera,
                &draw_2d::Quad::new(
                    AABB::point(vec2(0.0, -4.5)).extend_symmetric(vec2(3.0, 0.1)),
                    Rgba::BLACK,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &camera,
                &draw_2d::Quad::new(
                    AABB::point(vec2(-3.0 + 6.0 * self.best_progress, -4.5)).extend_uniform(0.3),
                    Rgba::new(0.0, 0.0, 0.0, 0.5),
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &camera,
                &draw_2d::Quad::new(
                    AABB::point(vec2(-3.0 + 6.0 * progress, -4.5)).extend_uniform(0.3),
                    Rgba::BLACK,
                ),
            );
        }
    }
}

impl geng::State for Game {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let mut test = String::new();
        let timer1 = Timer::new();
        let mut timer = Timer::new();
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(self.config.background_color), None, None);

        let level = self.levels.get(self.customization.postjam);

        self.draw_level_back(level, framebuffer);
        test += &format!("lvl back {}\n", timer.tick());
        self.draw_guys(framebuffer);
        test += &format!("guys {}\n", timer.tick());
        self.draw_level_front(level, framebuffer);
        test += &format!("lvl front {}\n", timer.tick());
        self.draw_farticles(framebuffer);
        test += &format!("farticles {}\n", timer.tick());
        self.draw_level_editor(framebuffer);
        test += &format!("editor {}\n", timer.tick());
        self.draw_customizer(framebuffer);
        test += &format!("customizer {}\n", timer.tick());
        self.draw_leaderboard(framebuffer);
        test += &format!("lb {}\n", timer.tick());
        self.draw_progress(framebuffer);
        test += &format!("progress {}\n", timer.tick());

        if timer1.elapsed() > 0.100 {
            println!("{}", test);
        }
    }

    fn fixed_update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        if self.my_guy.is_none() || !self.guys.get(&self.my_guy.unwrap()).unwrap().finished {
            self.simulation_time += delta_time;
        }
        for time in self.remote_simulation_times.values_mut() {
            *time += delta_time;
        }
        self.update_my_guy_input();
        self.update_guys(delta_time);
        self.update_farticles(delta_time);
    }

    fn update(&mut self, delta_time: f64) {
        // self.volume = self.assets.config.volume;
        if self.geng.window().is_key_pressed(geng::Key::PageUp) {
            self.volume += delta_time as f32 * 0.5;
        }
        if self.geng.window().is_key_pressed(geng::Key::PageDown) {
            self.volume -= delta_time as f32 * 0.5;
        }
        self.volume = self.volume.clamp(0.0, 1.0);
        if self.customization.postjam {
            self.new_music.set_volume(self.volume as f64);
            self.old_music.set_volume(0.0);
        } else {
            self.old_music.set_volume(self.volume as f64);
            self.new_music.set_volume(0.0);
        }
        self.emotes.retain(|&(t, ..)| t >= self.real_time - 1.0);
        let delta_time = delta_time as f32;
        self.real_time += delta_time;

        let mut target_center = self.camera.center;
        if let Some(id) = self.my_guy {
            let guy = self.guys.get(&id).unwrap();
            target_center = guy.pos;
            if self.show_customizer {
                target_center.x += 1.0;
            }
        } else if let Some(id) = self.follow {
            if let Some(guy) = self.guys.get(&id) {
                target_center = guy.pos;
            }
        }
        self.camera.center += (target_center - self.camera.center) * (delta_time * 5.0).min(1.0);

        if self.editor.is_none() {
            // let target_fov = if self.show_customizer { 2.0 } else { 6.0 };
            // self.camera.fov += (target_fov - self.camera.fov) * delta_time;
        }

        if let Some(editor) = &mut self.editor {
            editor.update(&mut self.levels, delta_time);
        }

        self.handle_connection();
        self.update_remote();

        if let Some(id) = self.my_guy {
            let guy = self.guys.get_mut(&id).unwrap();
            guy.name = self.customization.name.clone();
            guy.colors = self.customization.colors.clone();
            guy.postjam = self.customization.postjam;
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        self.handle_event_editor(&event);
        self.handle_customizer_event(&event);
        match event {
            geng::Event::MouseMove { position, .. }
                if self
                    .geng
                    .window()
                    .is_button_pressed(geng::MouseButton::Middle) =>
            {
                let old_pos = self
                    .camera
                    .screen_to_world(self.framebuffer_size, self.prev_mouse_pos.map(|x| x as f32));
                let new_pos = self
                    .camera
                    .screen_to_world(self.framebuffer_size, position.map(|x| x as f32));
                self.camera.center += old_pos - new_pos;
            }
            geng::Event::MouseDown {
                position,
                button: geng::MouseButton::Left,
            } if self.my_guy.is_none() && self.editor.is_none() => {
                let pos = self
                    .camera
                    .screen_to_world(self.framebuffer_size, position.map(|x| x as f32));
                if let Some(guy) = self
                    .guys
                    .iter()
                    .min_by_key(|guy| r32((guy.pos - pos).len()))
                {
                    if (guy.pos - pos).len() < self.assets.config.guy_radius {
                        self.follow = Some(guy.id);
                    }
                }
            }
            geng::Event::MouseDown {
                button: geng::MouseButton::Right,
                ..
            } => {
                self.follow = None;
            }
            geng::Event::Wheel { delta } if self.opt.editor => {
                self.camera.fov = (self.camera.fov * 1.01f32.powf(-delta as f32)).clamp(1.0, 30.0);
            }
            geng::Event::KeyDown { key: geng::Key::R }
                if self.geng.window().is_key_pressed(geng::Key::LCtrl) =>
            {
                self.respawn_my_guy();
            }
            geng::Event::KeyDown { key: geng::Key::H } if !self.show_customizer => {
                self.show_names = !self.show_names;
            }
            geng::Event::KeyDown { key: geng::Key::L } if !self.show_customizer => {
                if self.customization.postjam {
                    self.show_leaderboard = !self.show_leaderboard;
                }
            }
            geng::Event::KeyDown {
                key: geng::Key::Num1,
            } => self.connection.send(ClientMessage::Emote(0)),
            geng::Event::KeyDown {
                key: geng::Key::Num2,
            } => self.connection.send(ClientMessage::Emote(1)),
            geng::Event::KeyDown {
                key: geng::Key::Num3,
            } => self.connection.send(ClientMessage::Emote(2)),
            geng::Event::KeyDown {
                key: geng::Key::Num4,
            } => self.connection.send(ClientMessage::Emote(3)),
            _ => {}
        }
        self.prev_mouse_pos = self.geng.window().mouse_pos();
    }
}
