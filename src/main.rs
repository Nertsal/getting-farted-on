// TODO: write the rest of this comment
use geng::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
mod server;
mod ui;

type Connection = geng::net::client::Connection<ServerMessage, ClientMessage>;

#[derive(Clone)]
enum UiMessage {
    Play,
    RandomizeSkin,
}

use noise::NoiseFn;

pub const EPS: f32 = 1e-9;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Ping,
    Update(f32, Guy),
    Despawn,
    Emote(usize),
    ForceReset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Pong,
    ForceReset,
    ClientId(Id),
    UpdateGuy(f32, Guy),
    Despawn(Id),
    Emote(Id, usize),
}

pub const CONTROLS_LEFT: [geng::Key; 2] = [geng::Key::A, geng::Key::Left];
pub const CONTROLS_RIGHT: [geng::Key; 2] = [geng::Key::D, geng::Key::Right];
pub const CONTROLS_FORCE_FART: [geng::Key; 3] = [geng::Key::W, geng::Key::Up, geng::Key::Space];

#[derive(geng::Assets, Deserialize, Clone, Debug)]
#[asset(json)]
pub struct Config {
    volume: f32,
    snap_distance: f32,
    guy_radius: f32,
    angular_acceleration: f32,
    gravity: f32,
    max_angular_speed: f32, // TODO: maybe?
    fart_strength: f32,
    auto_fart_interval: f32,
    force_fart_interval: f32,
    fart_color: Rgba<f32>,
    farticle_w: f32,
    farticle_size: f32,
    farticle_count: usize,
    farticle_additional_vel: f32,
    background_color: Rgba<f32>,
}

#[derive(Deref)]
pub struct Texture(#[deref] ugli::Texture);

impl std::borrow::Borrow<ugli::Texture> for Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}
impl std::borrow::Borrow<ugli::Texture> for &'_ Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}

impl geng::LoadAsset for Texture {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let texture = <ugli::Texture as geng::LoadAsset>::load(geng, path);
        async move {
            let mut texture = texture.await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Texture(texture))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}

#[derive(Deserialize)]
pub struct SurfaceParams {
    pub bounciness: f32,
    pub friction: f32,
    pub front: bool,
    pub back: bool,
}

#[derive(Deserialize)]
pub struct BackgroundParams {
    #[serde(default)]
    pub friction: f32,
}

pub struct SurfaceAssets {
    pub name: String,
    pub params: SurfaceParams,
    pub front_texture: Option<Texture>,
    pub back_texture: Option<Texture>,
}

#[derive(geng::Assets)]
pub struct GuyAssets {
    pub cheeks: Texture,
    pub eyes: Texture,
    pub skin: Texture,
    pub clothes_top: Texture,
    pub clothes_bottom: Texture,
    pub hair: Texture,
}

fn load_surface_assets(
    geng: &Geng,
    path: &std::path::Path,
) -> geng::AssetFuture<HashMap<String, SurfaceAssets>> {
    let geng = geng.clone();
    let path = path.to_owned();
    async move {
        let json = <String as geng::LoadAsset>::load(&geng, &path.join("config.json")).await?;
        let config: std::collections::BTreeMap<String, SurfaceParams> =
            serde_json::from_str(&json).unwrap();
        future::join_all(config.into_iter().map(|(name, params)| {
            let geng = geng.clone();
            let path = path.clone();
            async move {
                let load = |file| {
                    let geng = geng.clone();
                    let path = path.clone();
                    async move {
                        let mut texture =
                            <Texture as geng::LoadAsset>::load(&geng, &path.join(file)).await?;
                        texture.0.set_wrap_mode(ugli::WrapMode::Repeat);
                        Ok::<_, anyhow::Error>(texture)
                    }
                };
                let mut back_texture = if params.back {
                    Some(load(format!("{}_back.png", name)).await?)
                } else {
                    None
                };
                let mut front_texture = if params.front {
                    Some(load(format!("{}_front.png", name)).await?)
                } else {
                    None
                };
                Ok((
                    name.clone(),
                    SurfaceAssets {
                        name,
                        params,
                        front_texture,
                        back_texture,
                    },
                ))
            }
        }))
        .await
        .into_iter()
        .collect::<Result<_, anyhow::Error>>()
    }
    .boxed_local()
}

pub struct BackgroundAssets {
    pub name: String,
    pub params: BackgroundParams,
    pub texture: Texture,
}

fn load_background_assets(
    geng: &Geng,
    path: &std::path::Path,
) -> geng::AssetFuture<HashMap<String, BackgroundAssets>> {
    let geng = geng.clone();
    let path = path.to_owned();
    async move {
        let json = <String as geng::LoadAsset>::load(&geng, &path.join("config.json")).await?;
        let config: std::collections::BTreeMap<String, BackgroundParams> =
            serde_json::from_str(&json).unwrap();
        future::join_all(config.into_iter().map(|(name, params)| {
            let geng = geng.clone();
            let path = path.clone();
            async move {
                let mut texture =
                    <Texture as geng::LoadAsset>::load(&geng, &path.join(format!("{}.png", name)))
                        .await?;
                texture.0.set_wrap_mode(ugli::WrapMode::Repeat);
                Ok((
                    name.clone(),
                    BackgroundAssets {
                        name,
                        params,
                        texture,
                    },
                ))
            }
        }))
        .await
        .into_iter()
        .collect::<Result<_, anyhow::Error>>()
    }
    .boxed_local()
}

#[derive(geng::Assets)]
pub struct SfxAssets {
    #[asset(range = "1..=3", path = "fart/*.wav")]
    pub fart: Vec<geng::Sound>,
    pub fart_recharge: geng::Sound,
    #[asset(path = "music.mp3")]
    pub old_music: geng::Sound,
    #[asset(path = "KuviFart.wav")]
    pub new_music: geng::Sound,
}

fn load_font(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<geng::Font> {
    let geng = geng.clone();
    let path = path.to_owned();
    async move {
        let data = <Vec<u8> as geng::LoadAsset>::load(&geng, &path).await?;
        Ok(geng::Font::new(
            &geng,
            &data,
            geng::ttf::Options {
                pixel_size: 64.0,
                max_distance: 0.1,
            },
        )?)
    }
    .boxed_local()
}

#[derive(geng::Assets)]
pub struct Assets {
    pub config: Config,
    pub sfx: SfxAssets,
    pub guy: GuyAssets,
    #[asset(load_with = "load_surface_assets(&geng, &base_path.join(\"surfaces\"))")]
    pub surfaces: HashMap<String, SurfaceAssets>,
    #[asset(load_with = "load_background_assets(&geng, &base_path.join(\"background\"))")]
    pub background: HashMap<String, BackgroundAssets>,
    pub farticle: Texture,
    #[asset(load_with = "load_font(&geng, &base_path.join(\"Ludum-Dairy-0.2.0.ttf\"))")]
    pub font: geng::Font,
    pub closed_outhouse: Texture,
    pub golden_toilet: Texture,
    #[asset(
        range = "[\"poggers\", \"fuuuu\", \"kekw\", \"eesBoom\"].into_iter()",
        path = "emotes/*.png"
    )]
    pub emotes: Vec<Texture>,
}

impl Assets {
    pub fn process(&mut self) {
        self.sfx.old_music.looped = true;
        self.sfx.new_music.looped = true;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Surface {
    pub p1: Vec2<f32>,
    pub p2: Vec2<f32>,
    pub type_name: String,
}

fn zero_flow() -> Vec2<f32> {
    Vec2::ZERO
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackgroundTile {
    pub vertices: [Vec2<f32>; 3],
    #[serde(default = "zero_flow")]
    pub flow: Vec2<f32>,
    pub type_name: String,
}

impl Surface {
    pub fn vector_from(&self, point: Vec2<f32>) -> Vec2<f32> {
        if Vec2::dot(point - self.p1, self.p2 - self.p1) < 0.0 {
            return self.p1 - point;
        }
        if Vec2::dot(point - self.p2, self.p1 - self.p2) < 0.0 {
            return self.p2 - point;
        }
        let n = (self.p2 - self.p1).rotate_90();
        // dot(point + n * t - p1, n) = 0
        // dot(point - p1, n) + dot(n, n) * t = 0
        let t = Vec2::dot(self.p1 - point, n) / Vec2::dot(n, n);
        n * t
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Level {
    pub spawn_point: Vec2<f32>,
    pub finish_point: Vec2<f32>,
    pub surfaces: Vec<Surface>,
    pub background_tiles: Vec<BackgroundTile>,
    pub expected_path: Vec<Vec2<f32>>,
}

impl Level {
    pub fn empty() -> Self {
        Self {
            spawn_point: Vec2::ZERO,
            finish_point: Vec2::ZERO,
            surfaces: vec![],
            background_tiles: vec![],
            expected_path: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Input {
    pub roll_direction: f32, // -1 to +1
    pub force_fart: bool,
}

pub type Id = i32;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuyColors {
    pub top: Rgba<f32>,
    pub bottom: Rgba<f32>,
    pub hair: Rgba<f32>,
    pub skin: Rgba<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, HasId)]
pub struct Guy {
    pub name: String,
    pub id: Id,
    pub pos: Vec2<f32>,
    pub vel: Vec2<f32>,
    pub rot: f32,
    pub w: f32,
    pub input: Input,
    pub auto_fart_timer: f32,
    pub force_fart_timer: f32,
    pub finished: bool,
    pub colors: GuyColors,
    pub postjam: bool,
    pub progress: f32,
    pub best_progress: f32,
    pub best_time: Option<f32>,
}

impl Guy {
    pub fn new(id: Id, pos: Vec2<f32>, rng: bool) -> Self {
        let random_hue = || {
            let hue = global_rng().gen_range(0.0..1.0);
            Hsva::new(hue, 1.0, 1.0, 1.0).into()
        };
        Self {
            name: "".to_owned(),
            id,
            pos: pos
                + if rng {
                    vec2(global_rng().gen_range(-1.0..=1.0), 0.0)
                } else {
                    Vec2::ZERO
                },
            vel: Vec2::ZERO,
            rot: if rng {
                global_rng().gen_range(-1.0..=1.0)
            } else {
                0.0
            },
            w: 0.0,
            input: Input::default(),
            auto_fart_timer: 0.0,
            force_fart_timer: 0.0,
            finished: false,
            colors: GuyColors {
                top: random_hue(),
                bottom: random_hue(),
                hair: random_hue(),
                skin: {
                    let tone = global_rng().gen_range(0.5..1.0);
                    Rgba::new(tone, tone, tone, 1.0)
                },
            },
            postjam: false,
            progress: 0.0,
            best_progress: 0.0,
            best_time: None,
        }
    }
}

struct EditorState {
    next_autosave: f32,
    start_drag: Option<Vec2<f32>>,
    face_points: Vec<Vec2<f32>>,
    selected_surface: String,
    selected_background: String,
    wind_drag: Option<(usize, Vec2<f32>)>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            next_autosave: 0.0,
            start_drag: None,
            face_points: vec![],
            selected_surface: "".to_owned(),
            selected_background: "".to_owned(),
            wind_drag: None,
        }
    }
}

pub struct Farticle {
    pub pos: Vec2<f32>,
    pub vel: Vec2<f32>,
    pub color: Rgba<f32>,
    pub rot: f32,
    pub w: f32,
    pub t: f32,
}

pub struct Game {
    emotes: Vec<(f32, Id, usize)>,
    best_progress: f32,
    framebuffer_size: Vec2<f32>,
    prev_mouse_pos: Vec2<f64>,
    geng: Geng,
    config: Config,
    assets: Rc<Assets>,
    camera: geng::Camera2d,
    levels: (Level, Level),
    editor: Option<EditorState>,
    guys: Collection<Guy>,
    my_guy: Option<Id>,
    simulation_time: f32,
    remote_simulation_times: HashMap<Id, f32>,
    remote_updates: HashMap<Id, std::collections::VecDeque<(f32, Guy)>>,
    real_time: f32,
    noise: noise::OpenSimplex,
    opt: Opt,
    farticles: Vec<Farticle>,
    volume: f32,
    client_id: Id,
    connection: Connection,
    customization: Guy,
    ui_controller: ui::Controller,
    buttons: Vec<ui::Button<UiMessage>>,
    show_customizer: bool,
    old_music: geng::SoundEffect,
    new_music: geng::SoundEffect,
    show_names: bool,
    show_leaderboard: bool,
    follow: Option<Id>,
    tas: Tas,
    tas_replay: Option<f32>,
}

/// For performing Tool Assisted Speedruns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tas {
    paused: bool,
    time: f32,
    time_scale: f32,
    rewind_time: f32,
    last_rewind: usize,
    timeline: Vec<(f32, Guy)>,
    save_states: Vec<(f32, Guy)>,
}

impl Drop for Game {
    fn drop(&mut self) {
        self.save_level();
    }
}

impl Default for Tas {
    fn default() -> Self {
        Self {
            paused: true,
            time: 0.0,
            time_scale: 1.0,
            rewind_time: 0.0,
            last_rewind: 0,
            timeline: Vec::new(),
            save_states: Vec::new(),
        }
    }
}

impl Game {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        levels: (Level, Level),
        opt: Opt,
        client_id: Id,
        connection: Connection,
    ) -> Self {
        let mut my_guy = None;
        let tas: Tas = if !cfg!(target_arch = "wasm32") {
            match std::fs::File::open("tas.json") {
                Ok(file) => {
                    let reader = std::io::BufReader::new(file);
                    match serde_json::from_reader::<_, Tas>(reader) {
                        Ok(tas) => {
                            info!("Sucessfully loaded tas");
                            my_guy = tas.timeline.last().map(|(_, state)| state.clone());
                            tas
                        }
                        Err(err) => {
                            error!("Failed to read TAS state: {err}");
                            default()
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to open tas.json: {err}");
                    default()
                }
            }
        } else {
            default()
        };

        let mut result = Self {
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
            show_leaderboard: opt.postjam,
            follow: None,
            tas,
            tas_replay: None,
        };
        if !opt.editor {
            result.my_guy = Some(client_id);
            let my_guy = match my_guy {
                Some(mut guy) => {
                    guy.id = client_id;
                    guy
                }
                None => Guy::new(
                    client_id,
                    if result.customization.postjam {
                        result.levels.1.spawn_point
                    } else {
                        result.levels.0.spawn_point
                    },
                    !result.customization.postjam,
                ),
            };
            result.guys.insert(my_guy);
        }
        result
    }

    pub fn snapped_cursor_position(&self) -> Vec2<f32> {
        self.snap_position(self.camera.screen_to_world(
            self.framebuffer_size,
            self.geng.window().mouse_pos().map(|x| x as f32),
        ))
    }

    pub fn snap_position(&self, pos: Vec2<f32>) -> Vec2<f32> {
        let closest_point = itertools::chain![
            self.levels
                .1
                .surfaces
                .iter()
                .flat_map(|surface| [surface.p1, surface.p2]),
            self.levels
                .1
                .background_tiles
                .iter()
                .flat_map(|tile| tile.vertices)
        ]
        .filter(|&p| (pos - p).len() < self.config.snap_distance)
        .min_by_key(|&p| r32((pos - p).len()));
        closest_point.unwrap_or(pos)
    }

    pub fn draw_guys(&self, framebuffer: &mut ugli::Framebuffer) {
        for guy in itertools::chain![
            self.guys.iter().filter(|guy| guy.id != self.client_id),
            self.guys.iter().filter(|guy| guy.id == self.client_id),
        ] {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(
                    &self.assets.guy.clothes_bottom,
                    guy.colors.bottom,
                )
                .scale_uniform(self.config.guy_radius)
                .transform(Mat3::rotate(guy.rot))
                .translate(guy.pos),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(&self.assets.guy.clothes_top, guy.colors.top)
                    .scale_uniform(self.config.guy_radius)
                    .transform(Mat3::rotate(guy.rot))
                    .translate(guy.pos),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(&self.assets.guy.hair, guy.colors.hair)
                    .scale_uniform(self.config.guy_radius)
                    .transform(Mat3::rotate(guy.rot))
                    .translate(guy.pos),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(&self.assets.guy.skin, guy.colors.skin)
                    .scale_uniform(self.config.guy_radius)
                    .transform(Mat3::rotate(guy.rot))
                    .translate(guy.pos),
            );
            let autofart_progress = guy.auto_fart_timer / self.config.auto_fart_interval;
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(&self.assets.guy.eyes, {
                    let k = 0.8;
                    let t = ((autofart_progress - k) / (1.0 - k)).clamp(0.0, 1.0) * 0.5;
                    Rgba::new(1.0, 1.0 - t, 1.0 - t, 1.0)
                })
                .translate(vec2(self.noise(10.0), self.noise(10.0)) * 0.1 * autofart_progress)
                .scale_uniform(self.config.guy_radius * (0.8 + 0.6 * autofart_progress))
                .transform(Mat3::rotate(guy.rot))
                .translate(guy.pos),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(
                    &self.assets.guy.cheeks,
                    Rgba {
                        a: (0.5 + 1.0 * autofart_progress).min(1.0),
                        ..guy.colors.skin
                    },
                )
                .translate(vec2(self.noise(10.0), self.noise(10.0)) * 0.1 * autofart_progress)
                .scale_uniform(self.config.guy_radius * (0.8 + 0.7 * autofart_progress))
                .transform(Mat3::rotate(guy.rot))
                .translate(guy.pos),
            );
            if Some(guy.id) == self.my_guy
                || (self.show_names && (!self.customization.postjam || guy.postjam))
            {
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    &guy.name,
                    guy.pos + vec2(0.0, self.config.guy_radius * 1.1),
                    geng::TextAlign::CENTER,
                    0.1,
                    Rgba::BLACK,
                );
            }
        }
        for &(_, id, emote) in &self.emotes {
            if let Some(guy) = self.guys.get(&id) {
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &draw_2d::TexturedQuad::unit(&self.assets.emotes[emote])
                        .scale_uniform(0.1)
                        .translate(guy.pos + vec2(0.0, self.config.guy_radius * 2.0)),
                );
            }
        }
    }

    pub fn draw_level_impl(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        texture: impl Fn(&SurfaceAssets) -> Option<&Texture>,
    ) {
        let level = if self.customization.postjam {
            &self.levels.1
        } else {
            &self.levels.0
        };
        for surface in &level.surfaces {
            let assets = &self.assets.surfaces[&surface.type_name];
            let texture = match texture(assets) {
                Some(texture) => texture,
                None => continue,
            };
            let normal = (surface.p2 - surface.p1).normalize().rotate_90();
            let len = (surface.p2 - surface.p1).len();
            let height = texture.size().y as f32 / texture.size().x as f32;
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedPolygon::new(
                    vec![
                        draw_2d::TexturedVertex {
                            a_pos: surface.p1,
                            a_color: Rgba::WHITE,
                            a_vt: vec2(0.0, 0.0),
                        },
                        draw_2d::TexturedVertex {
                            a_pos: surface.p2,
                            a_color: Rgba::WHITE,
                            a_vt: vec2(len, 0.0),
                        },
                        draw_2d::TexturedVertex {
                            a_pos: surface.p2 + normal * height,
                            a_color: Rgba::WHITE,
                            a_vt: vec2(len, 1.0),
                        },
                        draw_2d::TexturedVertex {
                            a_pos: surface.p1 + normal * height,
                            a_color: Rgba::WHITE,
                            a_vt: vec2(0.0, 1.0),
                        },
                    ],
                    texture,
                ),
            );
        }
    }

    pub fn draw_level_back(&self, framebuffer: &mut ugli::Framebuffer) {
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::unit(&self.assets.closed_outhouse).translate(
                if self.customization.postjam {
                    self.levels.1.spawn_point
                } else {
                    self.levels.0.spawn_point
                },
            ),
        );
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::unit(&self.assets.golden_toilet)
                .translate(self.levels.0.finish_point),
        );
        self.draw_level_impl(framebuffer, |assets| assets.back_texture.as_ref());
    }

    pub fn draw_level_front(&self, framebuffer: &mut ugli::Framebuffer) {
        let level = if self.customization.postjam {
            &self.levels.1
        } else {
            &self.levels.0
        };
        for tile in &level.background_tiles {
            let assets = &self.assets.background[&tile.type_name];
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedPolygon::new(
                    tile.vertices
                        .into_iter()
                        .map(|v| draw_2d::TexturedVertex {
                            a_pos: v,
                            a_color: Rgba::WHITE,
                            a_vt: v - tile.flow * self.simulation_time,
                        })
                        .collect(),
                    &assets.texture,
                ),
            );
        }
        self.draw_level_impl(framebuffer, |assets| assets.front_texture.as_ref());
    }

    pub fn find_hovered_surface(&self) -> Option<usize> {
        let cursor = self.camera.screen_to_world(
            self.framebuffer_size,
            self.geng.window().mouse_pos().map(|x| x as f32),
        );
        self.levels
            .1
            .surfaces
            .iter()
            .enumerate()
            .filter(|(_index, surface)| {
                surface.vector_from(cursor).len() < self.config.snap_distance
            })
            .min_by_key(|(_index, surface)| r32(surface.vector_from(cursor).len()))
            .map(|(index, _surface)| index)
    }

    pub fn find_hovered_background(&self) -> Option<usize> {
        let p = self.camera.screen_to_world(
            self.framebuffer_size,
            self.geng.window().mouse_pos().map(|x| x as f32),
        );
        'tile_loop: for (index, tile) in self.levels.1.background_tiles.iter().enumerate() {
            for i in 0..3 {
                let p1 = tile.vertices[i];
                let p2 = tile.vertices[(i + 1) % 3];
                if Vec2::skew(p2 - p1, p - p1) < 0.0 {
                    continue 'tile_loop;
                }
            }
            return Some(index);
        }
        None
    }

    pub fn draw_level_editor(&self, framebuffer: &mut ugli::Framebuffer) {
        if let Some(editor) = &self.editor {
            if let Some(p1) = editor.start_drag {
                let p2 = self.snapped_cursor_position();
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &draw_2d::Segment::new(
                        Segment::new(p1, p2),
                        0.1,
                        Rgba::new(1.0, 1.0, 1.0, 0.5),
                    ),
                );
            }
            if let Some(index) = self.find_hovered_surface() {
                let surface = &self.levels.1.surfaces[index];
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &draw_2d::Segment::new(
                        Segment::new(surface.p1, surface.p2),
                        0.2,
                        Rgba::new(1.0, 0.0, 0.0, 0.5),
                    ),
                );
            }
            if let Some(index) = self.find_hovered_background() {
                let tile = &self.levels.1.background_tiles[index];
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &draw_2d::Polygon::new(tile.vertices.into(), Rgba::new(0.0, 0.0, 1.0, 0.5)),
                );
            }
            for &p in &editor.face_points {
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &draw_2d::Quad::new(
                        AABB::point(p).extend_uniform(0.1),
                        Rgba::new(0.0, 1.0, 0.0, 0.5),
                    ),
                );
            }
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::new(
                    AABB::point(self.snapped_cursor_position()).extend_uniform(0.1),
                    Rgba::new(1.0, 0.0, 0.0, 0.5),
                ),
            );

            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::new(
                    AABB::point(self.levels.1.spawn_point).extend_uniform(0.1),
                    Rgba::new(1.0, 0.8, 0.8, 0.5),
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::new(
                    AABB::point(self.levels.1.finish_point).extend_uniform(0.1),
                    Rgba::new(1.0, 0.0, 0.0, 0.5),
                ),
            );

            for (i, &p) in self.levels.1.expected_path.iter().enumerate() {
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    &i.to_string(),
                    p,
                    geng::TextAlign::CENTER,
                    0.1,
                    Rgba::new(0.0, 0.0, 0.0, 0.5),
                );
            }

            if let Some((_, start)) = editor.wind_drag {
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &draw_2d::Segment::new(
                        Segment::new(
                            start,
                            self.camera.screen_to_world(
                                self.framebuffer_size,
                                self.geng.window().mouse_pos().map(|x| x as f32),
                            ),
                        ),
                        0.2,
                        Rgba::new(1.0, 0.0, 0.0, 0.5),
                    ),
                );
            }
        }
    }

    pub fn update_my_guy_input(&mut self) {
        if self.show_customizer {
            return;
        }
        let my_guy = match self.my_guy.map(|id| self.guys.get_mut(&id).unwrap()) {
            Some(guy) => guy,
            None => return,
        };
        if self.tas_replay.is_none() {
            let new_input = Input {
                roll_direction: {
                    let mut direction = 0.0;
                    if CONTROLS_LEFT
                        .iter()
                        .any(|&key| self.geng.window().is_key_pressed(key))
                    {
                        direction += 1.0;
                    }
                    if CONTROLS_RIGHT
                        .iter()
                        .any(|&key| self.geng.window().is_key_pressed(key))
                    {
                        direction -= 1.0;
                    }
                    direction
                },
                force_fart: CONTROLS_FORCE_FART
                    .iter()
                    .any(|&key| self.geng.window().is_key_pressed(key)),
            };
            if my_guy.input != new_input {
                my_guy.input = new_input;
                self.connection
                    .send(ClientMessage::Update(self.simulation_time, my_guy.clone()));
            }
        } else {
            self.connection
                .send(ClientMessage::Update(self.simulation_time, my_guy.clone()));
        }
    }

    pub fn update_guys(&mut self, mut delta_time: f32) {
        for guy in &mut self.guys {
            if (guy.pos - self.levels.0.finish_point).len() < 1.5 {
                guy.finished = true;
            }

            if let Some(time) = self.tas_replay {
                if Some(guy.id) == self.my_guy {
                    if let Some((_, state)) = self
                        .tas
                        .timeline
                        .iter()
                        .skip_while(|(t, _)| *t < time)
                        .next()
                        .or(self.tas.timeline.last())
                    {
                        let colors = guy.colors.clone();
                        let name = guy.name.clone();
                        let id = guy.id;
                        *guy = state.clone();
                        guy.id = id;
                        guy.colors = colors;
                        guy.name = name;
                    }
                }
            }

            if guy.finished {
                guy.auto_fart_timer = 0.0;
                guy.force_fart_timer = 0.0;
                guy.rot -= delta_time;
                guy.pos = self.levels.0.finish_point
                    + (guy.pos - self.levels.0.finish_point)
                        .normalize_or_zero()
                        .rotate(delta_time)
                        * 1.0;
                continue;
            }

            if self.tas_replay.is_none() && Some(guy.id) == self.my_guy {
                self.tas.rewind_time = self.tas.time;
                if self.tas.paused {
                    continue;
                }
                if self.tas.last_rewind + 1 < self.tas.timeline.len() {
                    self.tas.timeline.drain(self.tas.last_rewind + 1..);
                }
                self.tas.timeline.push((self.tas.time, guy.clone()));
                self.tas.last_rewind = self.tas.timeline.len();
                delta_time *= self.tas.time_scale;
                self.tas.time += delta_time;
            }

            guy.w += (guy.input.roll_direction.clamp(-1.0, 1.0)
                * self.config.angular_acceleration
                * delta_time)
                .clamp(
                    -(guy.w + self.config.max_angular_speed).max(0.0),
                    (self.config.max_angular_speed - guy.w).max(0.0),
                );
            guy.vel.y -= self.config.gravity * delta_time;

            let mut farts = 0;
            guy.auto_fart_timer += delta_time;
            if guy.auto_fart_timer >= self.config.auto_fart_interval {
                guy.auto_fart_timer = 0.0;
                farts += 1;
            }
            let could_force_fart = guy.force_fart_timer >= self.config.force_fart_interval;
            guy.force_fart_timer += delta_time;
            if guy.force_fart_timer >= self.config.force_fart_interval && guy.input.force_fart {
                farts += 1;
                guy.force_fart_timer = 0.0;
            }
            if !could_force_fart && guy.force_fart_timer >= self.config.force_fart_interval {
                if Some(guy.id) == self.my_guy {
                    let mut effect = self.assets.sfx.fart_recharge.effect();
                    effect.set_volume(self.volume as f64 * 0.5);
                    effect.play();
                }
            }
            for _ in 0..farts {
                for _ in 0..self.config.farticle_count {
                    self.farticles.push(Farticle {
                        pos: guy.pos,
                        vel: guy.vel
                            + vec2(
                                global_rng().gen_range(0.0..=self.config.farticle_additional_vel),
                                0.0,
                            )
                            .rotate(global_rng().gen_range(0.0..=2.0 * f32::PI)),
                        rot: global_rng().gen_range(0.0..2.0 * f32::PI),
                        w: global_rng().gen_range(-self.config.farticle_w..=self.config.farticle_w),
                        color: self.config.fart_color,
                        t: 1.0,
                    });
                }
                guy.vel += vec2(0.0, self.config.fart_strength).rotate(guy.rot);
                let mut effect = self
                    .assets
                    .sfx
                    .fart
                    .choose(&mut global_rng())
                    .unwrap()
                    .effect();
                effect.set_volume(
                    (self.volume * (1.0 - (guy.pos - self.camera.center).len() / self.camera.fov))
                        .clamp(0.0, 1.0) as f64,
                );
                effect.play();
            }

            guy.pos += guy.vel * delta_time;
            guy.rot += guy.w * delta_time;

            struct Collision<'a> {
                penetration: f32,
                normal: Vec2<f32>,
                surface_params: &'a SurfaceParams,
            }

            let mut collision_to_resolve = None;
            let level = if self.customization.postjam {
                &self.levels.1
            } else {
                &self.levels.0
            };
            for surface in &level.surfaces {
                let v = surface.vector_from(guy.pos);
                let penetration = self.config.guy_radius - v.len();
                if penetration > EPS && Vec2::dot(v, guy.vel) > 0.0 {
                    let collision = Collision {
                        penetration,
                        normal: -v.normalize_or_zero(),
                        surface_params: &self.assets.surfaces[&surface.type_name].params,
                    };
                    collision_to_resolve =
                        std::cmp::max_by_key(collision_to_resolve, Some(collision), |collision| {
                            r32(match collision {
                                Some(collision) => collision.penetration,
                                None => -1.0,
                            })
                        });
                }
            }
            if self.customization.postjam {
                'tile_loop: for (index, tile) in self.levels.1.background_tiles.iter().enumerate() {
                    for i in 0..3 {
                        let p1 = tile.vertices[i];
                        let p2 = tile.vertices[(i + 1) % 3];
                        if Vec2::skew(p2 - p1, guy.pos - p1) < 0.0 {
                            continue 'tile_loop;
                        }
                    }
                    let mut relative_vel = guy.vel - tile.flow;
                    let n = tile.flow.rotate_90().normalize_or_zero();
                    relative_vel -= n * Vec2::dot(n, relative_vel); // TODO: not always wanted, another param?
                    let params = &self.assets.background[&tile.type_name].params;
                    let force = -relative_vel * params.friction;
                    guy.vel += force * delta_time;
                }
            }
            if let Some(collision) = collision_to_resolve {
                guy.pos += collision.normal * collision.penetration;
                let normal_vel = Vec2::dot(guy.vel, collision.normal);
                let tangent = collision.normal.rotate_90();
                let tangent_vel = Vec2::dot(guy.vel, tangent) - guy.w * self.config.guy_radius;
                guy.vel -=
                    collision.normal * normal_vel * (1.0 + collision.surface_params.bounciness);
                let max_friction_impulse = normal_vel.abs() * collision.surface_params.friction;
                let friction_impulse = -tangent_vel.clamp_abs(max_friction_impulse);
                guy.vel += tangent * friction_impulse;
                guy.w -= friction_impulse / self.config.guy_radius;
            }
        }
    }

    #[track_caller]
    pub fn noise(&self, frequency: f32) -> f32 {
        let caller = std::panic::Location::caller();
        let phase = caller.line() as f64 * 1000.0 + caller.column() as f64;
        self.noise.get([(self.real_time * frequency) as f64, phase]) as f32
    }

    pub fn handle_event_editor(&mut self, event: &geng::Event) {
        if self.opt.editor
            && matches!(
                event,
                geng::Event::KeyDown {
                    key: geng::Key::Tab
                }
            )
        {
            if self.editor.is_none() {
                self.editor = Some(EditorState::new());
            } else {
                self.editor = None;
            }
        }
        if self.editor.is_none() {
            return;
        }
        let cursor_pos = self.snapped_cursor_position();
        let editor = self.editor.as_mut().unwrap();

        if !self.assets.surfaces.contains_key(&editor.selected_surface) {
            editor.selected_surface = self.assets.surfaces.keys().next().unwrap().clone();
        }
        if !self
            .assets
            .background
            .contains_key(&editor.selected_background)
        {
            editor.selected_background = self.assets.background.keys().next().unwrap().clone();
        }

        match event {
            geng::Event::MouseDown {
                button: geng::MouseButton::Left,
                ..
            } => {
                if let Some(editor) = &mut self.editor {
                    editor.start_drag = Some(cursor_pos);
                }
            }
            geng::Event::MouseUp {
                button: geng::MouseButton::Left,
                ..
            } => {
                let p2 = cursor_pos;

                if let Some(p1) = editor.start_drag.take() {
                    if (p1 - p2).len() > self.config.snap_distance {
                        self.levels.1.surfaces.push(Surface {
                            p1,
                            p2,
                            type_name: editor.selected_surface.clone(),
                        });
                    }
                }
            }
            geng::Event::MouseDown {
                button: geng::MouseButton::Right,
                ..
            } => {
                if let Some(index) = self.find_hovered_surface() {
                    self.levels.1.surfaces.remove(index);
                }
            }
            geng::Event::KeyUp { key } => match key {
                geng::Key::W => {
                    if let Some((index, start)) = editor.wind_drag.take() {
                        let to = self.camera.screen_to_world(
                            self.framebuffer_size,
                            self.geng.window().mouse_pos().map(|x| x as f32),
                        );
                        self.levels.1.background_tiles[index].flow = to - start;
                    }
                }
                _ => {}
            },
            geng::Event::KeyDown { key } => match key {
                geng::Key::W => {
                    if editor.wind_drag.is_none() {
                        self.editor.as_mut().unwrap().wind_drag =
                            self.find_hovered_background().map(|index| {
                                (
                                    index,
                                    self.camera.screen_to_world(
                                        self.framebuffer_size,
                                        self.geng.window().mouse_pos().map(|x| x as f32),
                                    ),
                                )
                            });
                    }
                }
                geng::Key::F => {
                    editor.face_points.push(cursor_pos);
                    if editor.face_points.len() == 3 {
                        let mut vertices: [Vec2<f32>; 3] =
                            mem::take(&mut editor.face_points).try_into().unwrap();
                        if Vec2::skew(vertices[1] - vertices[0], vertices[2] - vertices[0]) < 0.0 {
                            vertices.reverse();
                        }
                        self.levels.1.background_tiles.push(BackgroundTile {
                            vertices,
                            flow: Vec2::ZERO,
                            type_name: editor.selected_background.clone(),
                        });
                    }
                }
                geng::Key::D => {
                    if let Some(index) = self.find_hovered_background() {
                        self.levels.1.background_tiles.remove(index);
                    }
                }
                geng::Key::C => {
                    editor.face_points.clear();
                }
                geng::Key::R => {
                    if !self.geng.window().is_key_pressed(geng::Key::LCtrl) {
                        if let Some(id) = self.my_guy.take() {
                            self.connection.send(ClientMessage::Despawn);
                            self.guys.remove(&id);
                        } else {
                            self.my_guy = Some(self.client_id);
                            self.guys
                                .insert(Guy::new(self.client_id, cursor_pos, false));
                        }
                    }
                }
                geng::Key::P => {
                    self.levels.1.spawn_point = self.camera.screen_to_world(
                        self.framebuffer_size,
                        self.geng.window().mouse_pos().map(|x| x as f32),
                    );
                }
                geng::Key::I => {
                    let level = if self.customization.postjam {
                        &mut self.levels.1
                    } else {
                        &mut self.levels.0
                    };
                    level.expected_path.push(self.camera.screen_to_world(
                        self.framebuffer_size,
                        self.geng.window().mouse_pos().map(|x| x as f32),
                    ));
                }
                geng::Key::Backspace => {
                    let level = if self.customization.postjam {
                        &mut self.levels.1
                    } else {
                        &mut self.levels.0
                    };
                    level.expected_path.pop();
                }
                geng::Key::K => {
                    // self.level.finish_point = self.camera.screen_to_world(
                    //     self.framebuffer_size,
                    //     self.geng.window().mouse_pos().map(|x| x as f32),
                    // );
                }
                geng::Key::Z => {
                    let mut options: Vec<&String> = self.assets.surfaces.keys().collect();
                    options.sort();
                    let idx = options
                        .iter()
                        .position(|&s| s == &editor.selected_surface)
                        .unwrap_or(0);
                    editor.selected_surface = options[(idx + 1) % options.len()].clone();
                }
                geng::Key::X => {
                    let mut options: Vec<&String> = self.assets.background.keys().collect();
                    options.sort();
                    let idx = options
                        .iter()
                        .position(|&s| s == &editor.selected_background)
                        .unwrap_or(0);
                    editor.selected_background = options[(idx + 1) % options.len()].clone();
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn save_level(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.editor.is_some() {
            serde_json::to_writer_pretty(
                std::fs::File::create(static_path().join("new_level.json")).unwrap(),
                &self.levels.1,
            )
            .unwrap();
            info!("LVL SAVED");
        }
    }

    pub fn update_farticles(&mut self, delta_time: f32) {
        for farticle in &mut self.farticles {
            farticle.t -= delta_time;
            farticle.pos += farticle.vel * delta_time;
            farticle.rot += farticle.w * delta_time;

            let level = if self.customization.postjam {
                &self.levels.1
            } else {
                &self.levels.0
            };
            for surface in &level.surfaces {
                let v = surface.vector_from(farticle.pos);
                let penetration = self.config.farticle_size / 2.0 - v.len();
                if penetration > EPS && Vec2::dot(v, farticle.vel) > 0.0 {
                    let normal = -v.normalize_or_zero();
                    farticle.pos += normal * penetration;
                    farticle.vel -= normal * Vec2::dot(farticle.vel, normal);
                }
            }
        }
        self.farticles.retain(|farticle| farticle.t > 0.0);
    }

    pub fn draw_farticles(&self, framebuffer: &mut ugli::Framebuffer) {
        for farticle in &self.farticles {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(
                    &self.assets.farticle,
                    Rgba {
                        a: farticle.color.a * farticle.t,
                        ..farticle.color
                    },
                )
                .transform(Mat3::rotate(farticle.rot))
                .scale_uniform(self.config.farticle_size)
                .translate(farticle.pos),
            )
        }
    }

    pub fn handle_connection(&mut self) {
        let messages: Vec<ServerMessage> = self.connection.new_messages().collect();
        for message in messages {
            match message {
                ServerMessage::ForceReset => {
                    if self.my_guy.is_some() {
                        // COPYPASTA mmmmmmm
                        let new_guy = Guy::new(
                            self.client_id,
                            if self.customization.postjam {
                                self.levels.1.spawn_point
                            } else {
                                self.levels.0.spawn_point
                            },
                            !self.customization.postjam,
                        );
                        if self.my_guy.is_none() {
                            self.my_guy = Some(self.client_id);
                        }
                        self.guys.insert(new_guy);
                        self.simulation_time = 0.0;
                        self.connection.send(ClientMessage::Despawn);
                    }
                }
                ServerMessage::Pong => {
                    self.connection.send(ClientMessage::Ping);
                    if let Some(id) = self.my_guy {
                        let guy = self.guys.get(&id).unwrap();
                        self.connection
                            .send(ClientMessage::Update(self.simulation_time, guy.clone()));
                    }
                }
                ServerMessage::ClientId(_) => unreachable!(),
                ServerMessage::UpdateGuy(t, guy) => {
                    if !self.remote_simulation_times.contains_key(&guy.id) {
                        self.remote_simulation_times.insert(guy.id, t - 1.0);
                    }
                    self.remote_updates
                        .entry(guy.id)
                        .or_default()
                        .push_back((t, guy));
                }
                ServerMessage::Despawn(id) => {
                    self.guys.remove(&id);
                    self.remote_simulation_times.remove(&id);
                    if let Some(updates) = self.remote_updates.get_mut(&id) {
                        updates.clear();
                    }
                }
                ServerMessage::Emote(id, emote) => {
                    self.emotes.retain(|&(_, x, _)| x != id);
                    self.emotes.push((self.real_time, id, emote));
                }
            }
        }
    }

    fn update_remote(&mut self) {
        for (&id, updates) in &mut self.remote_updates {
            let current_simulation_time = match self.remote_simulation_times.get(&id) {
                Some(x) => *x,
                None => continue,
            };
            if let Some(update) = updates.back() {
                if (update.0 - current_simulation_time).abs() > 5.0 {
                    updates.clear();
                    self.remote_simulation_times.remove(&id);
                    self.guys.remove(&id);
                    continue;
                }
            }
            while let Some(update) = updates.front() {
                if (update.0 - current_simulation_time).abs() > 5.0 {
                    updates.clear();
                    self.remote_simulation_times.remove(&id);
                    self.guys.remove(&id);
                    break;
                }
                if update.0 <= current_simulation_time {
                    let update = updates.pop_front().unwrap().1;
                    self.guys.insert(update);
                } else {
                    break;
                }
            }
        }
    }

    pub fn draw_customizer(&mut self, framebuffer: &mut ugli::Framebuffer) {
        if !self.show_customizer {
            return;
        }
        let camera = geng::Camera2d {
            center: Vec2::ZERO,
            rotation: 0.0,
            fov: 10.0,
        };
        self.ui_controller
            .draw(framebuffer, &camera, self.buttons.clone());
        if self.customization.name.is_empty() {
            self.assets.font.draw(
                framebuffer,
                &camera,
                "type your name",
                vec2(0.0, 3.0),
                geng::TextAlign::CENTER,
                1.0,
                Rgba::new(0.5, 0.5, 1.0, 0.5),
            );
            self.assets.font.draw(
                framebuffer,
                &camera,
                "yes just type it",
                vec2(0.0, 2.0),
                geng::TextAlign::CENTER,
                1.0,
                Rgba::new(0.5, 0.5, 1.0, 0.5),
            );
        } else {
            self.assets.font.draw(
                framebuffer,
                &camera,
                &self.customization.name,
                vec2(0.0, 3.0),
                geng::TextAlign::CENTER,
                1.0,
                Rgba::new(0.5, 0.5, 1.0, 1.0),
            );
        }
    }

    fn handle_customizer_event(&mut self, event: &geng::Event) {
        if !self.show_customizer {
            return;
        }
        for msg in self.ui_controller.handle_event(event, self.buttons.clone()) {
            match msg {
                UiMessage::Play => {
                    self.show_customizer = false;
                }
                UiMessage::RandomizeSkin => {
                    self.customization.colors =
                        Guy::new(-1, Vec2::ZERO, !self.customization.postjam).colors;
                }
            }
        }
        match event {
            geng::Event::KeyDown { key } => {
                let s = format!("{:?}", key);
                if s.len() == 1 && self.customization.name.len() < 15 {
                    self.customization.name.push_str(&s);
                }
                if *key == geng::Key::Backspace {
                    self.customization.name.pop();
                }
                if self.customization.name.to_lowercase() == "postjamplease" {
                    self.customization.postjam = true;
                    self.show_leaderboard = true;
                }
                if self.customization.name.to_lowercase() == "iamoutfrost" {
                    self.customization.postjam = true;
                    self.show_leaderboard = true;
                    self.opt.editor = true;
                    if let Some(id) = self.my_guy.take() {
                        self.connection.send(ClientMessage::Despawn);
                        self.guys.remove(&id);
                    }
                }
            }
            _ => {}
        }
    }

    fn draw_leaderboard(&self, framebuffer: &mut ugli::Framebuffer) {
        if !self.show_leaderboard {
            return;
        }
        let mut guys: Vec<&Guy> = self.guys.iter().filter(|guy| guy.postjam).collect();
        guys.sort_by(|a, b| match (a.best_time, b.best_time) {
            (Some(a), Some(b)) => a.partial_cmp(&b).unwrap(),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a
                .best_progress
                .partial_cmp(&b.best_progress)
                .unwrap()
                .reverse(),
        });
        let mut camera = geng::Camera2d {
            center: Vec2::ZERO,
            rotation: 0.0,
            fov: 40.0,
        };
        camera.center.x += camera.fov * self.framebuffer_size.x / self.framebuffer_size.y / 2.0;
        for (place, guy) in guys.into_iter().enumerate() {
            let place = place + 1;
            let name = &guy.name;
            let progress = (guy.progress * 100.0).round() as i32;
            let mut text = format!("#{place}: {name} - {progress}% (");
            if let Some(time) = guy.best_time {
                let millis = (time * 1000.0).round() as i32;
                let seconds = millis / 1000;
                let millis = millis % 1000;
                let minutes = seconds / 60;
                let seconds = seconds % 60;
                let hours = minutes / 60;
                let minutes = minutes % 60;
                if hours != 0 {
                    text += &format!("{}:", hours);
                }
                if minutes != 0 {
                    text += &format!("{}:", minutes);
                }
                text += &format!("{}.{}", seconds, millis);
            } else {
                text += &format!("{}%", (guy.best_progress * 100.0).round() as i32);
            }
            text.push(')');
            self.geng.default_font().draw(
                framebuffer,
                &camera,
                &text,
                vec2(1.0, camera.fov / 2.0 - place as f32),
                geng::TextAlign::LEFT,
                1.0,
                Rgba::BLACK,
            );
        }
    }

    fn handle_event_cheats(&mut self, event: &geng::Event) {
        let window = self.geng.window();
        match event {
            geng::Event::Wheel { delta } => {
                let delta = *delta as f32 * 0.002;
                self.tas.time_scale = (self.tas.time_scale + delta).clamp(0.0, 2.0);
            }
            geng::Event::KeyDown { key } => match key {
                geng::Key::R if window.is_key_pressed(geng::Key::LShift) => match self.tas_replay {
                    Some(_) => self.tas_replay = None,
                    None => self.tas_replay = Some(0.0),
                },
                geng::Key::P => {
                    self.tas.paused = !self.tas.paused;
                }
                geng::Key::G => {
                    if let Some(guy) = self.my_guy.and_then(|id| self.guys.get(&id)) {
                        self.tas.save_states.push((self.tas.time, guy.clone()));
                    }
                }
                geng::Key::T => {
                    if let Some((time, state)) = self.tas.save_states.last() {
                        if let Some(guy) = self.my_guy.and_then(|id| self.guys.get_mut(&id)) {
                            self.tas.time = *time;
                            let colors = guy.colors.clone();
                            let name = guy.name.clone();
                            let id = guy.id;
                            *guy = state.clone();
                            guy.id = id;
                            guy.colors = colors;
                            guy.name = name;
                        }
                    }
                }
                geng::Key::N => {
                    self.tas.time_scale = (self.tas.time_scale - 0.1).clamp(0.0, 2.0);
                }
                geng::Key::M => {
                    self.tas.time_scale = (self.tas.time_scale + 0.1).clamp(0.0, 2.0);
                }
                geng::Key::Num1 => self.tas.time_scale = 0.1,
                geng::Key::Num2 => self.tas.time_scale = 0.2,
                geng::Key::Num3 => self.tas.time_scale = 0.3,
                geng::Key::Num4 => self.tas.time_scale = 0.4,
                geng::Key::Num5 => self.tas.time_scale = 0.5,
                geng::Key::Num6 => self.tas.time_scale = 0.6,
                geng::Key::Num7 => self.tas.time_scale = 0.7,
                geng::Key::Num8 => self.tas.time_scale = 0.8,
                geng::Key::Num9 => self.tas.time_scale = 0.9,
                geng::Key::Num0 => self.tas.time_scale = 1.0,
                geng::Key::S if window.is_key_pressed(geng::Key::LCtrl) => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        match std::fs::File::create("tas.json") {
                            Ok(file) => {
                                let writer = std::io::BufWriter::new(file);
                                if let Err(err) = serde_json::to_writer(writer, &self.tas) {
                                    error!("Failed to save TAS state: {err}");
                                }
                                info!("Sucessfully saved tas");
                            }
                            Err(err) => {
                                error!("Failed to open tas.json: {err}");
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

impl geng::State for Game {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(self.config.background_color), None, None);

        self.draw_level_back(framebuffer);
        self.draw_guys(framebuffer);
        self.draw_level_front(framebuffer);
        self.draw_farticles(framebuffer);
        self.draw_level_editor(framebuffer);

        self.draw_customizer(framebuffer);

        self.draw_leaderboard(framebuffer);

        if !self.show_customizer {
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
                    let level = if self.customization.postjam {
                        &self.levels.1
                    } else {
                        &self.levels.0
                    };
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
                if guy.finished && self.simulation_time < guy.best_time.unwrap_or(1e9) {
                    guy.best_time = Some(self.simulation_time);
                }
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
                        AABB::point(vec2(-3.0 + 6.0 * self.best_progress, -4.5))
                            .extend_uniform(0.3),
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

                let text = format!("Time scale: {:.2} ", self.tas.time_scale);
                self.assets.font.draw(
                    framebuffer,
                    &camera,
                    &text,
                    vec2(0.0, 4.3),
                    geng::TextAlign::CENTER,
                    0.5,
                    Rgba::BLACK,
                );
                if self.tas.paused {
                    let text = "Paused";
                    self.assets.font.draw(
                        framebuffer,
                        &camera,
                        &text,
                        vec2(0.0, 3.3),
                        geng::TextAlign::CENTER,
                        0.5,
                        Rgba::BLACK,
                    );
                }
                let mut time_text = String::from("TAS: ");
                let millis = (self.tas.time * 1000.0).round() as i32;
                let seconds = millis / 1000;
                let millis = millis % 1000;
                let minutes = seconds / 60;
                let seconds = seconds % 60;
                let hours = minutes / 60;
                let minutes = minutes % 60;
                if hours != 0 {
                    time_text += &format!("{}h ", hours);
                }
                if minutes != 0 {
                    time_text += &format!("{}m ", minutes);
                }
                time_text += &format!("{}s {:03}ms", seconds, millis);
                self.assets.font.draw(
                    framebuffer,
                    &camera,
                    &time_text,
                    vec2(-8.0, 4.3),
                    geng::TextAlign::LEFT,
                    0.5,
                    Rgba::BLACK,
                );
            }
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

        if let Some(time) = &mut self.tas_replay {
            *time += delta_time;
        } else if self.geng.window().is_key_pressed(geng::Key::K) {
            if let Some(guy) = self.my_guy.and_then(|id| self.guys.get_mut(&id)) {
                self.tas.rewind_time -= delta_time * self.tas.time_scale;
                while self.tas.time > self.tas.rewind_time {
                    let rewind = match self.tas.last_rewind.checked_sub(1) {
                        Some(x) => x,
                        None => break,
                    };
                    if let Some((time, state)) = self.tas.timeline.get(rewind) {
                        self.tas.last_rewind = rewind;
                        self.tas.time = *time;
                        *guy = state.clone();
                    } else {
                        break;
                    }
                }
                return;
            }
        } else if self.geng.window().is_key_pressed(geng::Key::L) {
            if let Some(guy) = self.my_guy.and_then(|id| self.guys.get_mut(&id)) {
                self.tas.rewind_time += delta_time * self.tas.time_scale;
                while self.tas.time < self.tas.rewind_time {
                    let rewind = self.tas.last_rewind + 1;
                    if let Some((time, state)) = self.tas.timeline.get(rewind) {
                        self.tas.last_rewind = rewind;
                        self.tas.time = *time;
                        *guy = state.clone();
                    } else {
                        break;
                    }
                }
                return;
            }
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
            editor.next_autosave -= delta_time;
            if editor.next_autosave < 0.0 {
                editor.next_autosave = 10.0;
                self.save_level();
            }
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
        self.handle_event_cheats(&event);

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
            geng::Event::KeyDown { key: geng::Key::S }
                if self.geng.window().is_key_pressed(geng::Key::LCtrl) =>
            {
                self.save_level();
            }
            geng::Event::KeyDown { key: geng::Key::R }
                if self.geng.window().is_key_pressed(geng::Key::LCtrl) =>
            {
                if self.my_guy.is_none() && self.editor.is_none() {
                    self.connection.send(ClientMessage::ForceReset);
                } else {
                    if !self.customization.postjam
                        || !self
                            .guys
                            .iter()
                            .any(|guy| guy.name.to_lowercase() == "pomo")
                    {
                        let new_guy = Guy::new(
                            self.client_id,
                            if self.customization.postjam {
                                self.levels.1.spawn_point
                            } else {
                                self.levels.0.spawn_point
                            },
                            !self.customization.postjam,
                        );
                        if self.my_guy.is_none() {
                            self.my_guy = Some(self.client_id);
                        }
                        self.guys.insert(new_guy);
                        self.simulation_time = 0.0;
                        self.connection.send(ClientMessage::Despawn);
                        self.tas = default();
                    }
                }
            }
            geng::Event::KeyDown { key: geng::Key::H } => {
                self.show_names = !self.show_names;
            }
            geng::Event::KeyDown { key: geng::Key::L } => {
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

#[derive(clap::Parser, Clone)]
pub struct Opt {
    #[clap(long)]
    pub editor: bool,
    #[clap(long)]
    pub server: Option<String>,
    #[clap(long)]
    pub connect: Option<String>,
    #[clap(long)]
    pub postjam: bool,
}

fn main() {
    geng::setup_panic_handler();
    let mut opt: Opt = program_args::parse();

    if opt.connect.is_none() && opt.server.is_none() {
        if cfg!(target_arch = "wasm32") {
            opt.connect = Some(
                option_env!("CONNECT")
                    .unwrap_or("ws://127.0.0.1:1155")
                    // .expect("Set CONNECT compile time env var")
                    .to_owned(),
            );
        } else {
            opt.server = Some("127.0.0.1:1155".to_owned());
            opt.connect = Some("ws://127.0.0.1:1155".to_owned());
        }
    }

    logger::init().unwrap();

    if opt.server.is_some() && opt.connect.is_none() {
        #[cfg(not(target_arch = "wasm32"))]
        server::Server::new(opt.server.as_deref().unwrap()).run();
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        let server = if let Some(addr) = &opt.server {
            let server = server::Server::new(addr);
            let server_handle = server.handle();
            let server_thread = std::thread::spawn(move || {
                server.run();
            });
            Some((server_handle, server_thread))
        } else {
            None
        };

        let geng = Geng::new_with(geng::ContextOptions {
            title: "LD51 - Getting Farted On".to_owned(),
            fixed_delta_time: 1.0 / 200.0,
            vsync: false,
            ..default()
        });
        let connection = geng::net::client::connect::<ServerMessage, ClientMessage>(
            opt.connect.as_deref().unwrap(),
        )
        .then(|connection| async move {
            let (message, mut connection) = connection.into_future().await;
            let id = match message {
                Some(ServerMessage::ClientId(id)) => id,
                _ => unreachable!(),
            };
            connection.send(ClientMessage::Ping);
            (id, connection)
        });
        let state = geng::LoadingScreen::new(
            &geng,
            geng::EmptyLoadingScreen,
            future::join(
                future::join(
                    <Assets as geng::LoadAsset>::load(&geng, &static_path()),
                    future::join(
                        <String as geng::LoadAsset>::load(
                            &geng,
                            &static_path().join("old_level.json"),
                        ),
                        <String as geng::LoadAsset>::load(
                            &geng,
                            &static_path().join("new_level.json"),
                        ),
                    ),
                ),
                connection,
            ),
            {
                let geng = geng.clone();
                move |((assets, (old_level, new_level)), (client_id, connection))| {
                    let mut assets = assets.expect("Failed to load assets");
                    assets.process();
                    let old_level = serde_json::from_str(&old_level.unwrap()).unwrap();
                    let new_level = serde_json::from_str(&new_level.unwrap()).unwrap();
                    let assets = Rc::new(assets);
                    Game::new(
                        &geng,
                        &assets,
                        (old_level, new_level),
                        opt,
                        client_id,
                        connection,
                    )
                }
            },
        );
        geng::run(&geng, state);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some((server_handle, server_thread)) = server {
            server_handle.shutdown();
            server_thread.join().unwrap();
        }
    }
}
