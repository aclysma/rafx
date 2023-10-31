// This file generated automatically by hydrate-codegen. Do not make manual edits. Use include!() to place these types in the intended location.
#[derive(Default)]
pub struct Vec3Record(PropertyPath);

impl Field for Vec3Record {
    fn new(property_path: PropertyPath) -> Self {
        Vec3Record(property_path)
    }
}

impl Record for Vec3Record {
    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl Vec3Record {
    pub fn x(&self) -> F32Field {
        F32Field::new(self.0.push("x"))
    }

    pub fn y(&self) -> F32Field {
        F32Field::new(self.0.push("y"))
    }

    pub fn z(&self) -> F32Field {
        F32Field::new(self.0.push("z"))
    }
}
#[derive(Default)]
pub struct Vec4Record(PropertyPath);

impl Field for Vec4Record {
    fn new(property_path: PropertyPath) -> Self {
        Vec4Record(property_path)
    }
}

impl Record for Vec4Record {
    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl Vec4Record {
    pub fn w(&self) -> F32Field {
        F32Field::new(self.0.push("w"))
    }

    pub fn x(&self) -> F32Field {
        F32Field::new(self.0.push("x"))
    }

    pub fn y(&self) -> F32Field {
        F32Field::new(self.0.push("y"))
    }

    pub fn z(&self) -> F32Field {
        F32Field::new(self.0.push("z"))
    }
}
