#[cfg(not(target_arch = "wasm32"))]
fn main() {
    demo_web::main_native()
}
