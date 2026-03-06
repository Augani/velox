use velox::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Velox Demo")
        .power_policy(PowerPolicy::Adaptive)
        .window(
            WindowConfig::new("main")
                .title("Velox Demo — Main Window")
                .size(1200, 800)
                .min_size(400, 300),
        )
        .window(
            WindowConfig::new("inspector")
                .title("Velox Demo — Inspector")
                .size(400, 600),
        )
        .run()
}
