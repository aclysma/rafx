use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{
    Error, ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter,
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;
use gltf::image::Data as GltfImageData;
use gltf::buffer::Data as GltfBufferData;
use crate::image_importer::ImageAsset;
use fnv::FnvHashMap;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
enum GltfObjectId {
    Name(String),
    Index(usize)
}

#[derive(TypeUuid, Serialize, Deserialize)]
#[uuid = "130a91a8-ba80-4cad-9bce-848326b234c7"]
pub struct GltfMaterialAsset {
    base_color: [f32;4]
}

//TODO: It might not make practical sense to have an overall GLTF asset in the long run, probably
// would produce separate image, mesh, prefab assets
#[derive(TypeUuid, Serialize, Deserialize)]
#[uuid = "122e4e01-d3d3-4e99-8725-c6fcee30ff1a"]
pub struct GltfAsset {
    base_color: [f32;4]
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "807c83b3-c24c-4123-9580-5f9c426260b4"]
struct GltfImporterState {
    asset_uuid: Option<AssetUuid>,

    // Asset UUIDs for imported image by name
    image_asset_uuids: FnvHashMap<GltfObjectId, AssetUuid>,
}

#[derive(TypeUuid)]
#[uuid = "fc9ae812-110d-4daf-9223-e87b40966c6b"]
struct GltfImporter;
impl Importer for GltfImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        5
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = GltfImporterState;

    /// Reads the given bytes and produces assets.
    fn import(
        &self,
        source: &mut Read,
        options: Self::Options,
        state: &mut Self::State,
    ) -> atelier_assets::importer::Result<ImporterValue> {
        //
        // Get the asset UUID, or create a new UUID if this is a new gltf file
        //
        let gltf_asset_uuid = state
            .asset_uuid
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

        state.asset_uuid = Some(gltf_asset_uuid);

        //
        // Load the GLTF file
        //
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        //let (doc, buffers, images) = gltf::import_slice(&bytes).unwrap();
        let result = gltf::import_slice(&bytes);
        if let Err(err) = result {
            log::error!("GLTF Import error: {:?}", err);
            return Err(Error::Boxed(Box::new(err)));
        }


        let (doc, buffers, images) = gltf::import_slice(&bytes).unwrap();

        let mut imported_assets = Vec::new();

        //
        // Fetch all the data
        //
        let images_to_import = extract_images_to_import(&doc, &buffers, &images);

        //
        // Create image assets
        //
        let mut image_index_to_uuid_lookup = vec![];
        for image_to_import in images_to_import {
            // Find the UUID associated with this image or create a new one
            let image_uuid = *state.image_asset_uuids.entry(image_to_import.id.clone())
                .or_insert_with(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));

            // Push the UUID into the list so that we have an O(1) lookup for image index to UUID
            image_index_to_uuid_lookup.push(image_uuid.clone());

            let mut search_tags : Vec<(String, Option<String>)> = vec![];
            if let GltfObjectId::Name(name) = &image_to_import.id {
                search_tags.push(("image_name".to_string(), Some(name.clone())));
            }

            // Create the asset
            imported_assets.push(ImportedAsset {
                id: image_uuid,
                search_tags,
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(image_to_import.image_asset),
            });
        }

        // //
        // let material_asset = GltfMaterialAsset {
        //     base_color: [1.0, 1.0, 1.0, 1.0]
        // };

        Ok(ImporterValue {
            assets: imported_assets,
        })
    }
}

struct ImageToImport {
    id: GltfObjectId,
    image_asset: ImageAsset,
}

fn extract_images_to_import(
    doc: &gltf::Document,
    buffers: &Vec<GltfBufferData>,
    images: &Vec<GltfImageData>
) -> Vec<ImageToImport> {
    let mut images_to_import = Vec::with_capacity(images.len());
    for image in doc.images() {
        let image_data = &images[image.index()];

        // Convert it to standard RGBA format
        use gltf::image::Format;
        use image::buffer::ConvertBuffer;
        let converted_image : image::RgbaImage = match image_data.format {
            Format::R8 => {
                image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::R8G8 => {
                image::ImageBuffer::<image::LumaA<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::R8G8B8 => {
                image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::R8G8B8A8 => {
                image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::B8G8R8 => {
                image::ImageBuffer::<image::Bgr<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::B8G8R8A8 => {
                image::ImageBuffer::<image::Bgra<u8>, Vec<u8>>::from_vec(image_data.width, image_data.height, image_data.pixels.clone()).unwrap().convert()
            },
            Format::R16 => {
                unimplemented!();
            },
            Format::R16G16 => {
                unimplemented!();
            },
            Format::R16G16B16 => {
                unimplemented!();
            },
            Format::R16G16B16A16 => {
                unimplemented!();
            },
        };

        let image_asset = crate::image_importer::ImageAsset {
            data: converted_image.to_vec(),
            width: image_data.width,
            height: image_data.height
        };
        let id = image.name().map(|s| GltfObjectId::Name(s.to_string())).unwrap_or(GltfObjectId::Index(image.index()));

        let image_to_import = ImageToImport {
            id,
            image_asset
        };

        // Verify that we iterate images in order so that our resulting assets are in order
        assert!(image.index() == images_to_import.len());
        println!(
            "Importing Texture Name: {:?} Index: {} Width: {} Height: {} Bytes: {}",
            image.name(),
            image.index(),
            image_to_import.image_asset.width,
            image_to_import.image_asset.height,
            image_to_import.image_asset.data.len()
        );

        images_to_import.push(image_to_import);
    }

    images_to_import
}


// make a macro to reduce duplication here :)
inventory::submit!(SourceFileImporter {
    extension: "gltf",
    instantiator: || Box::new(GltfImporter {}),
});
