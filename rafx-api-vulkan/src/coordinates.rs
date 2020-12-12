//! Defines physical and logical coordinates. This is heavily based on winit's design.

/// A size in raw pixels
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

impl PhysicalSize {
    pub fn new(
        width: u32,
        height: u32,
    ) -> Self {
        PhysicalSize { width, height }
    }

    pub fn to_logical(
        self,
        scale_factor: f64,
    ) -> LogicalSize {
        LogicalSize {
            width: (self.width as f64 * scale_factor).round() as u32,
            height: (self.height as f64 * scale_factor).round() as u32,
        }
    }
}

/// A size in raw pixels * a scaling factor. The scaling factor could be increased for hidpi
/// displays
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LogicalSize {
    pub width: u32,
    pub height: u32,
}

impl LogicalSize {
    pub fn new(
        width: u32,
        height: u32,
    ) -> Self {
        LogicalSize { width, height }
    }

    pub fn to_physical(
        self,
        scale_factor: f64,
    ) -> PhysicalSize {
        PhysicalSize {
            width: (self.width as f64 / scale_factor).round() as u32,
            height: (self.height as f64 / scale_factor).round() as u32,
        }
    }
}

/// A size that's either physical or logical.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Size {
    Physical(PhysicalSize),
    Logical(LogicalSize),
}

impl From<PhysicalSize> for Size {
    fn from(physical_size: PhysicalSize) -> Self {
        Size::Physical(physical_size)
    }
}

impl From<LogicalSize> for Size {
    fn from(logical_size: LogicalSize) -> Self {
        Size::Logical(logical_size)
    }
}

impl Size {
    pub fn new<S: Into<Size>>(size: S) -> Size {
        size.into()
    }

    pub fn to_logical(
        &self,
        scale_factor: f64,
    ) -> LogicalSize {
        match *self {
            Size::Physical(size) => size.to_logical(scale_factor),
            Size::Logical(size) => size,
        }
    }

    pub fn to_physical(
        &self,
        scale_factor: f64,
    ) -> PhysicalSize {
        match *self {
            Size::Physical(size) => size,
            Size::Logical(size) => size.to_physical(scale_factor),
        }
    }
}
