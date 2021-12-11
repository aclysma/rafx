use crate::assets::image::{
    ImageAssetData, ImageAssetDataLayer, ImageAssetDataMipLevel, ImageAssetDataPayload,
};
use crate::{ImageAssetDataFormat, ImageAssetDataPayloadSubresources};
use ddsfile::DxgiFormat;
use distill::importer::{Error, ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use rafx_api::RafxResourceType;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "5eaeedae-8319-48f3-a50b-039f0613ec61"]
pub struct DdsImageImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "c1cedd9b-5f28-42e0-afc7-77891d7cadb4"]
pub struct DdsImageImporter;

impl Importer for DdsImageImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        1
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = DdsImageImporterState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = DdsImageImporterState(Some(id));
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let dds = ddsfile::Dds::read(&mut &bytes[..]).map_err(|e| Error::Boxed(Box::new(e)))?;

        let format = if let Some(dxgi_format) = dds.get_dxgi_format() {
            match dxgi_format {
                //DxgiFormat::BC1_Typeless => {}
                DxgiFormat::BC1_UNorm => ImageAssetDataFormat::BC1_UNorm_Linear,
                DxgiFormat::BC1_UNorm_sRGB => ImageAssetDataFormat::BC1_UNorm_Srgb,
                //DxgiFormat::BC2_Typeless => {}
                DxgiFormat::BC2_UNorm => ImageAssetDataFormat::BC2_UNorm_Linear,
                DxgiFormat::BC2_UNorm_sRGB => ImageAssetDataFormat::BC2_UNorm_Srgb,
                //DxgiFormat::BC3_Typeless => {}
                DxgiFormat::BC3_UNorm => ImageAssetDataFormat::BC3_UNorm_Linear,
                DxgiFormat::BC3_UNorm_sRGB => ImageAssetDataFormat::BC3_UNorm_Srgb,
                //DxgiFormat::BC4_Typeless => {}
                DxgiFormat::BC4_UNorm => ImageAssetDataFormat::BC4_UNorm,
                DxgiFormat::BC4_SNorm => ImageAssetDataFormat::BC4_SNorm,
                //DxgiFormat::BC5_Typeless => {}
                DxgiFormat::BC5_UNorm => ImageAssetDataFormat::BC5_UNorm,
                DxgiFormat::BC5_SNorm => ImageAssetDataFormat::BC5_SNorm,
                //DxgiFormat::BC6H_Typeless => {}
                DxgiFormat::BC6H_UF16 => ImageAssetDataFormat::BC6H_UFloat,
                DxgiFormat::BC6H_SF16 => ImageAssetDataFormat::BC6H_SFloat,
                //DxgiFormat::BC7_Typeless => {}
                DxgiFormat::BC7_UNorm => ImageAssetDataFormat::BC7_Unorm_Linear,
                DxgiFormat::BC7_UNorm_sRGB => ImageAssetDataFormat::BC7_Unorm_Srgb,
                _ => unimplemented!(),
            }
        } else {
            unimplemented!();
        };

        let width = dds.get_width();
        let height = dds.get_height();
        let array_layer_count = dds.get_num_array_layers();
        let mip_level_count = dds.get_num_mipmap_levels();

        if dds.get_depth() != 1 {
            unimplemented!("DDS importer only supports image depth = 1");
        }

        log::trace!(
            "w: {} h: {} layers: {} mips: {} format: {:?} dxgi_format: {:?} d3d_format: {:?}",
            width,
            height,
            array_layer_count,
            mip_level_count,
            format,
            dds.get_dxgi_format(),
            dds.get_d3d_format()
        );
        //println!("Import DDS texture: {:?}", dds);

        let mut layers_asset_data = Vec::with_capacity(array_layer_count as usize);
        for layer_index in 0..array_layer_count {
            let layer = dds.get_data(0).map_err(|e| Error::Boxed(Box::new(e)))?;

            let mut current_mipmap_size_bytes = dds.get_main_texture_size().unwrap() as usize;
            let min_mipmap_size_bytes = dds.get_min_mipmap_size_in_bytes() as usize;
            let mut offset_bytes = 0_usize;

            let mut mip_width = width;
            let mut mip_height = height;

            let mut mip_levels_asset_data = Vec::with_capacity(mip_level_count as usize);
            for mip_index in 0..mip_level_count {
                let mip_data: Vec<u8> = layer
                    [offset_bytes..(offset_bytes + current_mipmap_size_bytes)]
                    .iter()
                    .copied()
                    .collect();
                log::trace!(
                    "Gathered mip data {} {} {}",
                    layer_index,
                    mip_index,
                    mip_data.len()
                );

                mip_levels_asset_data.push(ImageAssetDataMipLevel {
                    width: mip_width,
                    height: mip_height,
                    bytes: mip_data,
                });

                offset_bytes += current_mipmap_size_bytes;
                current_mipmap_size_bytes /= 4;
                if current_mipmap_size_bytes < min_mipmap_size_bytes {
                    current_mipmap_size_bytes = min_mipmap_size_bytes;
                }

                mip_width /= 2;
                mip_height /= 2;
            }

            layers_asset_data.push(ImageAssetDataLayer {
                mip_levels: mip_levels_asset_data,
            });
        }

        let asset_data = ImageAssetData {
            width,
            height,
            format,
            generate_mips_at_runtime: false,
            resource_type: RafxResourceType::TEXTURE,
            data: ImageAssetDataPayload::Subresources(ImageAssetDataPayloadSubresources {
                layers: layers_asset_data,
            }),
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(asset_data),
            }],
        })
    }
}
