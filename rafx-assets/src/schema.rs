use hydrate_data::*;
use hydrate_model::{DataContainer, DataContainerMut, DataSetResult};

include!("schema_codegen.rs");

impl Vec3Record {
    pub fn set_vec3(
        &self,
        data_container: &mut DataContainerMut,
        value: [f32; 3],
    ) -> DataSetResult<()> {
        self.x().set(data_container, value[0])?;
        self.y().set(data_container, value[1])?;
        self.z().set(data_container, value[2])?;
        Ok(())
    }

    pub fn get_vec3(
        &self,
        data_container: &DataContainer,
    ) -> DataSetResult<[f32; 3]> {
        let x = self.x().get(data_container)?;
        let y = self.y().get(data_container)?;
        let z = self.z().get(data_container)?;
        Ok([x, y, z])
    }
}

impl Vec4Record {
    pub fn set_vec4(
        &self,
        data_container: &mut DataContainerMut,
        value: [f32; 4],
    ) -> DataSetResult<()> {
        self.x().set(data_container, value[0])?;
        self.y().set(data_container, value[1])?;
        self.z().set(data_container, value[2])?;
        self.w().set(data_container, value[3])?;
        Ok(())
    }

    pub fn get_vec4(
        &self,
        data_container: &DataContainer,
    ) -> DataSetResult<[f32; 4]> {
        let x = self.x().get(data_container)?;
        let y = self.y().get(data_container)?;
        let z = self.z().get(data_container)?;
        let w = self.w().get(data_container)?;
        Ok([x, y, z, w])
    }
}
