use velox_window::WindowConfig;

#[test]
fn window_config_builder() {
    let config = WindowConfig::new("main")
        .title("Test Window")
        .size(1200, 800)
        .min_size(400, 300)
        .resizable(true);

    assert_eq!(config.id_label(), "main");
    assert_eq!(config.get_title(), "Test Window");
    assert_eq!(config.get_size(), (1200, 800));
    assert_eq!(config.get_min_size(), Some((400, 300)));
    assert!(config.is_resizable());
}

#[test]
fn window_config_defaults() {
    let config = WindowConfig::new("default");
    assert_eq!(config.get_title(), "Velox");
    assert_eq!(config.get_size(), (800, 600));
    assert_eq!(config.get_min_size(), None);
    assert!(config.is_resizable());
}

#[test]
fn window_config_max_size() {
    let config = WindowConfig::new("constrained").max_size(1920, 1080);
    assert_eq!(config.get_max_size(), Some((1920, 1080)));
}

#[test]
fn window_config_decorations() {
    let config = WindowConfig::new("borderless").decorations(false);
    assert!(!config.has_decorations());
}

#[test]
fn window_config_to_attributes() {
    let config = WindowConfig::new("test")
        .title("Attr Test")
        .min_size(100, 100)
        .max_size(2000, 2000);
    let attrs = config.to_window_attributes();
    assert_eq!(attrs.title, "Attr Test");
    assert!(attrs.min_inner_size.is_some());
    assert!(attrs.max_inner_size.is_some());
}
