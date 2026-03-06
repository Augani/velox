use velox_app::App;
use velox_runtime::power::PowerPolicy;
use velox_window::WindowConfig;

#[test]
fn app_builder_defaults() {
    let app = App::builder("test-app").build();
    assert_eq!(app.name(), "test-app");
    assert_eq!(app.power_policy(), PowerPolicy::Adaptive);
    assert!(app.window_configs().is_empty());
}

#[test]
fn app_builder_with_name() {
    let app = App::builder("my-app").build();
    assert_eq!(app.name(), "my-app");
}

#[test]
fn app_builder_with_power_policy() {
    let app = App::builder("test")
        .power_policy(PowerPolicy::Saving)
        .build();
    assert_eq!(app.power_policy(), PowerPolicy::Saving);
}

#[test]
fn app_builder_with_window() {
    let app = App::builder("test")
        .window(
            WindowConfig::new("main")
                .title("Main Window")
                .size(1024, 768),
        )
        .build();
    assert_eq!(app.window_configs().len(), 1);
    assert_eq!(app.window_configs()[0].id_label(), "main");
    assert_eq!(app.window_configs()[0].get_title(), "Main Window");
}

#[test]
fn app_builder_with_multiple_windows() {
    let app = App::builder("test")
        .window(WindowConfig::new("main").title("Main"))
        .window(
            WindowConfig::new("inspector")
                .title("Inspector")
                .size(400, 600),
        )
        .build();
    assert_eq!(app.window_configs().len(), 2);
    assert_eq!(app.window_configs()[0].id_label(), "main");
    assert_eq!(app.window_configs()[1].id_label(), "inspector");
}
