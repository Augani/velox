use velox_app::App;
use velox_window::WindowConfig;

#[test]
fn app_builder_single_window() {
    let app = App::new()
        .name("Test App")
        .window(WindowConfig::new("main").title("Test").size(800, 600));
    assert_eq!(app.window_configs().len(), 1);
}

#[test]
fn app_builder_multi_window() {
    let app = App::new()
        .name("Multi Window")
        .window(WindowConfig::new("main").title("Main"))
        .window(
            WindowConfig::new("inspector")
                .title("Inspector")
                .size(400, 600),
        );
    assert_eq!(app.window_configs().len(), 2);
}

#[test]
fn app_defaults() {
    let app = App::new();
    assert_eq!(app.window_configs().len(), 0);
}
