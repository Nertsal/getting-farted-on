use super::*;

mod cannon;
mod draw;
mod object;
mod portal;
mod progress;
mod surface;
mod tile;

pub use cannon::*;
pub use object::*;
pub use portal::*;
pub use surface::*;
pub use tile::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct LevelLayer {
    pub name: String,
    pub gameplay: bool,
    pub surfaces: Vec<Surface>,
    pub tiles: Vec<Tile>,
    pub objects: Vec<Object>,
    #[serde(default = "default_parallax")]
    pub parallax: vec2<f32>,
    #[serde(default)]
    pub reveal_radius: f32,
}

fn default_parallax() -> vec2<f32> {
    vec2(1.0, 1.0)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LevelInfo {
    pub spawn_point: vec2<f32>,
    pub finish_point: vec2<f32>,
    pub expected_path: Vec<Vec<vec2<f32>>>,
    pub layers: Vec<LevelLayer>,
    pub cannons: Vec<Cannon>,
    pub portals: Vec<Portal>,
    pub max_progress_distance: f32,
}

impl Default for LevelInfo {
    fn default() -> Self {
        Self {
            spawn_point: vec2::ZERO,
            finish_point: vec2::ZERO,
            expected_path: vec![],
            layers: vec![LevelLayer {
                name: "main".to_owned(),
                gameplay: true,
                surfaces: vec![],
                tiles: vec![],
                objects: vec![],
                parallax: default_parallax(),
                reveal_radius: 0.0,
            }],
            cannons: vec![],
            portals: vec![],
            max_progress_distance: 10.0,
        }
    }
}

impl LevelInfo {
    pub fn gameplay_surfaces(&self) -> impl Iterator<Item = &Surface> {
        self.layers
            .iter()
            .filter(|layer| layer.gameplay)
            .flat_map(|layer| &layer.surfaces)
    }

    pub fn gameplay_tiles(&self) -> impl Iterator<Item = &Tile> {
        self.layers
            .iter()
            .filter(|layer| layer.gameplay)
            .flat_map(|layer| &layer.tiles)
    }

    pub fn gameplay_objects(&self) -> impl Iterator<Item = &Object> {
        self.layers
            .iter()
            .filter(|layer| layer.gameplay)
            .flat_map(|layer| &layer.objects)
    }

    pub fn all_surfaces(&self) -> impl Iterator<Item = &Surface> {
        self.layers.iter().flat_map(|layer| &layer.surfaces)
    }

    pub fn all_tiles(&self) -> impl Iterator<Item = &Tile> {
        self.layers.iter().flat_map(|layer| &layer.tiles)
    }

    pub fn all_objects(&self) -> impl Iterator<Item = &Object> {
        self.layers.iter().flat_map(|layer| &layer.objects)
    }
}

#[derive(Deref)]
pub struct Level {
    path: std::path::PathBuf,
    #[deref]
    info: LevelInfo,
    mesh: RefCell<Option<draw::LevelMesh>>,
    history: Vec<LevelInfo>,
    history_index: usize,
    saved: bool,
}

impl Level {
    pub async fn load(path: impl AsRef<std::path::Path>, create_if_not_exist: bool) -> Self {
        let path = path.as_ref();
        let mut saved = true;
        let info: LevelInfo = match file::load_json(path).await {
            Ok(info) => info,
            Err(e) => {
                if !path.exists() && create_if_not_exist {
                    let info: LevelInfo = default();
                    saved = false;
                    info
                } else {
                    panic!("{e}");
                }
            }
        };
        Self {
            path: path.to_owned(),
            info,
            mesh: RefCell::new(None),
            history: vec![],
            history_index: 0,
            saved,
        }
    }
    pub fn info(&self) -> &LevelInfo {
        &self.info
    }
    pub fn modify(&mut self) -> &mut LevelInfo {
        *self.mesh.get_mut() = None;
        self.saved = false;
        self.history.truncate(self.history_index);
        self.history.push(self.info.clone());
        self.history_index += 1;
        &mut self.info
    }
    pub fn undo(&mut self) {
        if self.history_index > 0 {
            *self.mesh.get_mut() = None;
            self.saved = false;
            if self.history_index >= self.history.len() {
                assert!(self.history_index == self.history.len());
                self.history.push(self.info.clone());
            }
            self.history_index -= 1;
            self.info = self.history[self.history_index].clone();
        }
    }
    pub fn redo(&mut self) {
        if self.history_index + 1 < self.history.len() {
            *self.mesh.get_mut() = None;
            self.saved = false;
            self.history_index += 1;
            self.info = self.history[self.history_index].clone();
        }
    }
    pub fn save(&mut self) {
        if !mem::replace(&mut self.saved, true) {
            serde_json::to_writer_pretty(
                std::io::BufWriter::new(std::fs::File::create(&self.path).unwrap()),
                self.info(),
            )
            .unwrap();
        }
    }
}
