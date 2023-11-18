use hydrate_data::*;
use hydrate_pipeline::{DataContainerOwned, DataContainerRef, DataContainerRefMut, DataSetResult};
use std::cell::RefCell;
use std::rc::Rc;

include!("schema_codegen.rs");

impl Vec3Accessor {
    pub fn set_vec3(
        &self,
        data_container: &mut DataContainerRefMut,
        value: [f32; 3],
    ) -> DataSetResult<()> {
        self.x().set(data_container, value[0])?;
        self.y().set(data_container, value[1])?;
        self.z().set(data_container, value[2])?;
        Ok(())
    }

    pub fn get_vec3(
        &self,
        data_container: DataContainerRef,
    ) -> DataSetResult<[f32; 3]> {
        let x = self.x().get(data_container)?;
        let y = self.y().get(data_container)?;
        let z = self.z().get(data_container)?;
        Ok([x, y, z])
    }
}

impl Vec4Accessor {
    pub fn set_vec4(
        &self,
        data_container: &mut DataContainerRefMut,
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
        data_container: DataContainerRef,
    ) -> DataSetResult<[f32; 4]> {
        let x = self.x().get(data_container)?;
        let y = self.y().get(data_container)?;
        let z = self.z().get(data_container)?;
        let w = self.w().get(data_container)?;
        Ok([x, y, z, w])
    }
}

impl Vec3Owned {
    pub fn set_vec3(
        &self,
        value: [f32; 3],
    ) -> DataSetResult<()> {
        self.x().set(value[0])?;
        self.y().set(value[1])?;
        self.z().set(value[2])?;
        Ok(())
    }

    pub fn get_vec3(&self) -> DataSetResult<[f32; 3]> {
        let x = self.x().get()?;
        let y = self.y().get()?;
        let z = self.z().get()?;
        Ok([x, y, z])
    }
}

impl Vec4Owned {
    pub fn set_vec4(
        &self,
        value: [f32; 4],
    ) -> DataSetResult<()> {
        self.x().set(value[0])?;
        self.y().set(value[1])?;
        self.z().set(value[2])?;
        self.w().set(value[3])?;
        Ok(())
    }

    pub fn get_vec4(&self) -> DataSetResult<[f32; 4]> {
        let x = self.x().get()?;
        let y = self.y().get()?;
        let z = self.z().get()?;
        let w = self.w().get()?;
        Ok([x, y, z, w])
    }
}
