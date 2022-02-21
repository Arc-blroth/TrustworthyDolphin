use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::asset::{AssetIo, AssetIoError, BoxedFuture};
use bevy::prelude::{AssetServer, Plugin};

use crate::{App, IoTaskPool};

#[derive(Clone, Debug)]
pub struct EmbeddedAssetIo {
    dirs: HashMap<&'static Path, Vec<PathBuf>>,
    assets: HashMap<&'static Path, &'static [u8]>,
}

impl EmbeddedAssetIo {
    pub fn new(assets: HashMap<&'static Path, &'static [u8]>) -> Self {
        let mut dirs = HashMap::new();
        for asset in &assets {
            if let Some(parent) = asset.0.parent() {
                let directory = match dirs.get_mut(parent) {
                    Some(directory) => directory,
                    None => {
                        dirs.insert(parent, Vec::new());
                        dirs.get_mut(parent).unwrap()
                    }
                };
                directory.push(asset.0.to_path_buf());
            }
        }

        Self { dirs, assets }
    }
}

impl AssetIo for EmbeddedAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move {
            match self.assets.get(path) {
                Some(asset) => Ok((*asset).into()),
                None => Err(AssetIoError::NotFound(path.to_path_buf())),
            }
        })
    }

    fn read_directory(&self, path: &Path) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        match self.dirs.get(path) {
            Some(dir) => Ok(Box::new(dir.clone().into_iter())),
            None => Err(AssetIoError::NotFound(path.to_path_buf())),
        }
    }

    fn is_directory(&self, path: &Path) -> bool {
        self.dirs.contains_key(path)
    }

    fn watch_path_for_changes(&self, _path: &Path) -> Result<(), AssetIoError> {
        Ok(())
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        Ok(())
    }
}

pub struct EmbeddedAssetsPlugin;

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        let task_pool = app
            .world
            .get_resource::<IoTaskPool>()
            .expect("`IoTaskPool` resource not found.")
            .0
            .clone();
        let asset_io = app
            .world
            .get_resource::<EmbeddedAssetIo>()
            .expect("Missing `EmbeddedAssetIo` resource!")
            .clone();
        let asset_server = AssetServer::with_boxed_io(Box::new(asset_io), task_pool);
        app.insert_resource(asset_server);
    }

    fn name(&self) -> &str {
        "EmbeddedAssetsPlugin"
    }
}

pub macro include_assets($($asset:literal),*$(,)?) {{
    let mut assets = HashMap::new();
    $(
        assets.insert($asset.as_ref(), &include_bytes!(concat!("../assets/", $asset))[..]);
    )*
    EmbeddedAssetIo::new(assets)
}}
