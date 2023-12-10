use hydrate::editor::egui::Ui;
use hydrate::editor::inspector_system::*;
use hydrate::model::{Record, Schema, SchemaRecord, SchemaSet};
use rafx::assets::schema::*;

struct Vec3RecordInspector;

impl RecordInspector for Vec3RecordInspector {
    fn can_draw_as_single_value(&self) -> bool {
        true
    }

    fn draw_inspector_value(
        &self,
        ui: &mut Ui,
        ctx: InspectorContext,
    ) {
        ui.label("X");
        let field_path = ctx.property_path.push("x");
        draw_inspector_value(
            ui,
            InspectorContext {
                property_default_display_name: "x",
                property_path: &field_path,
                schema: &Schema::F32,
                ..ctx
            },
        );
        ui.label("Y");
        let field_path = ctx.property_path.push("y");
        draw_inspector_value(
            ui,
            InspectorContext {
                property_default_display_name: "y",
                property_path: &field_path,
                schema: &Schema::F32,
                ..ctx
            },
        );
        ui.label("Z");
        let field_path = ctx.property_path.push("z");
        draw_inspector_value(
            ui,
            InspectorContext {
                property_default_display_name: "z",
                property_path: &field_path,
                schema: &Schema::F32,
                ..ctx
            },
        );
    }
}

struct Vec4RecordInspector;

impl RecordInspector for Vec4RecordInspector {
    fn can_draw_as_single_value(&self) -> bool {
        true
    }

    fn draw_inspector_value(
        &self,
        ui: &mut Ui,
        ctx: InspectorContext,
    ) {
        ui.label("X");
        let field_path = ctx.property_path.push("x");
        draw_inspector_value(
            ui,
            InspectorContext {
                property_default_display_name: "x",
                property_path: &field_path,
                schema: &Schema::F32,
                ..ctx
            },
        );
        ui.label("Y");
        let field_path = ctx.property_path.push("y");
        draw_inspector_value(
            ui,
            InspectorContext {
                property_default_display_name: "y",
                property_path: &field_path,
                schema: &Schema::F32,
                ..ctx
            },
        );
        ui.label("Z");
        let field_path = ctx.property_path.push("z");
        draw_inspector_value(
            ui,
            InspectorContext {
                property_default_display_name: "z",
                property_path: &field_path,
                schema: &Schema::F32,
                ..ctx
            },
        );

        ui.label("W");
        let field_path = ctx.property_path.push("w");
        draw_inspector_value(
            ui,
            InspectorContext {
                property_default_display_name: "w",
                property_path: &field_path,
                schema: &Schema::F32,
                ..ctx
            },
        );
    }
}

pub fn register_inspectors(
    schema_set: &SchemaSet,
    inspector_registry: &mut InspectorRegistry,
) {
    inspector_registry.register_inspector::<Vec3Record>(schema_set, Vec3RecordInspector);
    inspector_registry.register_inspector::<Vec4Record>(schema_set, Vec4RecordInspector);
}
