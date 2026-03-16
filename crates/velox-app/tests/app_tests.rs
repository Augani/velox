use velox_app::App;
use velox_platform::PlatformClipboard;
use velox_style::{Theme, ThemeManager};
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

#[test]
fn app_builder_accepts_theme_manager() {
    let app = App::new()
        .theme_manager(ThemeManager::new(Theme::generated_default()))
        .window(WindowConfig::new("main").title("Main"));
    assert_eq!(app.window_configs().len(), 1);
}

#[test]
fn app_builder_accepts_platform_clipboard() {
    struct TestClipboard;

    impl PlatformClipboard for TestClipboard {
        fn read_text(&self) -> Option<String> {
            None
        }

        fn write_text(&self, _text: &str) {}
    }

    let app = App::new()
        .clipboard(TestClipboard)
        .window(WindowConfig::new("main").title("Main"));
    assert_eq!(app.window_configs().len(), 1);
}
