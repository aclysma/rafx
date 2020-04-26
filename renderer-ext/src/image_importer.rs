use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{Error, ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;

#[derive(TypeUuid, Serialize, Deserialize)]
#[uuid = "d4079e74-3ec9-4ebc-9b77-a87cafdfdada"]
// pub enum ImageAsset {
//     Rgb8(ImageBuf<u8, color::Rgb>),
//     // ...
// }
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "3c8367c8-45fb-40bb-a229-00e5e9c3fc70"]
struct ImageImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "720d636b-b79c-42d4-8f46-a2d8e1ada46e"]
struct ImageImporter;
impl Importer for ImageImporter {
    fn version_static() -> u32
        where
            Self: Sized,
    {
        2
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = ImageImporterState;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut Read,
        options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = ImageImporterState(Some(id));
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let decoded_image = image2::io::decode::<_, _, image2::Rgba>(&bytes).map_err(|e| Error::Boxed(Box::new(e)))?;

        let image_asset = ImageAsset {
            width: decoded_image.width() as u32,
            height: decoded_image.height() as u32,
            data: decoded_image.data().to_vec()
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(image_asset),
            }],
        })
    }
}
// make a macro to reduce duplication here :)
inventory::submit!(SourceFileImporter {
    extension: "png",
    instantiator: || Box::new(ImageImporter {}),
});
inventory::submit!(SourceFileImporter {
    extension: "jpg",
    instantiator: || Box::new(ImageImporter {}),
});
inventory::submit!(SourceFileImporter {
    extension: "jpeg",
    instantiator: || Box::new(ImageImporter {}),
});
inventory::submit!(SourceFileImporter {
    extension: "tga",
    instantiator: || Box::new(ImageImporter {}),
});