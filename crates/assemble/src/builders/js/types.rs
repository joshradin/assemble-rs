use rquickjs::{bind, Ctx, FromJs, HasRefs, IntoJs, Object, ObjectDef, RefsMarker, Value};

#[derive(Debug, Default, Clone, FromJs, IntoJs)]
pub struct Settings {
    root_project: JsProjectDescriptor,
}

#[derive(Debug, Default, Clone, FromJs, IntoJs)]
pub struct JsProjectDescriptor {
    name: String,
}
