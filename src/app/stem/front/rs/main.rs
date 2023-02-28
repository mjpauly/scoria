use front::App;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    log::debug!("App is starting");
    yew::Renderer::<App>::new().render();
}
