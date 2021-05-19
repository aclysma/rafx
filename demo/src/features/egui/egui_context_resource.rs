use crate::features::egui::internal::EguiManager;

/// Lightweight access to just the egui context (used to add UI elements)
pub struct EguiContextResource {
    pub(super) egui_manager: EguiManager,
}

impl EguiContextResource {
    pub fn context(&self) -> egui::CtxRef {
        self.egui_manager.context()
    }
}
