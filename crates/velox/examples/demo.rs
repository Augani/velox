use velox::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Velox Demo")
        .window(
            WindowConfig::new("main")
                .title("Velox — Main Window")
                .size(1200, 800),
        )
        .window(
            WindowConfig::new("inspector")
                .title("Velox — Inspector")
                .size(400, 600)
                .resizable(true),
        )
        .run()
}
