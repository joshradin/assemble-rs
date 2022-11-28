use assemble_js_plugin::Engine;
use rquickjs::{Context, Undefined};

#[test]
fn create_engine() {
    let mut engine = Engine::new();
    let mut ctx: Context = engine.new_context().expect("to create a new context");
    let ret = ctx
        .with(|ctx| {
            ctx.globals().set("hello", rquickjs::Undefined)?;
            ctx.eval::<(), _>("hello = \"hello world\";")?;
            ctx.globals().get::<_, String>("hello")
        })
        .expect("global hello not set to a string");
    assert_eq!(ret, "hello world");
}

#[test]
fn create_engine_with_decs() {
    let mut engine = Engine::new().with_declaration("hello", Undefined);
    let mut ctx: Context = engine.new_context().expect("to create a new context");
    let ret = ctx
        .with(|ctx| {
            ctx.eval::<(), _>("hello = \"hello world\";")?;
            ctx.globals().get::<_, String>("hello")
        })
        .expect("global hello not set to a string");
    assert_eq!(ret, "hello world");
}
