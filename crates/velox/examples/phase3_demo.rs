use velox::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Phase 3 Demo")
        .window(
            WindowConfig::new("main")
                .title("Velox — GPU Rendering")
                .size(1200, 800),
        )
        .run()
}
