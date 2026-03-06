use std::env;
use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ThemeSpec {
    name: String,
    palette: PaletteSpec,
    space: SpaceSpec,
    radius: RadiusSpec,
    typography: TypographySpec,
}

#[derive(Debug, Deserialize)]
struct PaletteSpec {
    background: [u8; 4],
    surface: [u8; 4],
    surface_alt: [u8; 4],
    text_primary: [u8; 4],
    text_muted: [u8; 4],
    accent: [u8; 4],
    accent_hover: [u8; 4],
    selection: [u8; 4],
}

#[derive(Debug, Deserialize)]
struct SpaceSpec {
    xs: f32,
    sm: f32,
    md: f32,
    lg: f32,
    xl: f32,
}

#[derive(Debug, Deserialize)]
struct RadiusSpec {
    sm: f32,
    md: f32,
    lg: f32,
}

#[derive(Debug, Deserialize)]
struct TypographySpec {
    body: f32,
    heading: f32,
    mono: f32,
}

fn main() {
    let spec_path = Path::new("tokens/default_theme.toml");
    println!("cargo:rerun-if-changed={}", spec_path.display());

    let src = fs::read_to_string(spec_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", spec_path.display()));
    let spec: ThemeSpec = toml::from_str(&src)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", spec_path.display()));

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let out_path = Path::new(&out_dir).join("generated_theme.rs");
    fs::write(&out_path, render_generated_theme(&spec))
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", out_path.display()));
}

fn render_generated_theme(spec: &ThemeSpec) -> String {
    let p = &spec.palette;
    let s = &spec.space;
    let r = &spec.radius;
    let t = &spec.typography;

    format!(
        r#"pub fn generated_default_theme() -> Theme {{
    Theme {{
        name: {name:?},
        palette: Palette {{
            background: ThemeColor::rgba({b0}, {b1}, {b2}, {b3}),
            surface: ThemeColor::rgba({s0}, {s1}, {s2}, {s3}),
            surface_alt: ThemeColor::rgba({sa0}, {sa1}, {sa2}, {sa3}),
            text_primary: ThemeColor::rgba({tp0}, {tp1}, {tp2}, {tp3}),
            text_muted: ThemeColor::rgba({tm0}, {tm1}, {tm2}, {tm3}),
            accent: ThemeColor::rgba({a0}, {a1}, {a2}, {a3}),
            accent_hover: ThemeColor::rgba({ah0}, {ah1}, {ah2}, {ah3}),
            selection: ThemeColor::rgba({sel0}, {sel1}, {sel2}, {sel3}),
        }},
        space: SpaceScale {{
            xs: Spacing::px({xs:?}),
            sm: Spacing::px({sm:?}),
            md: Spacing::px({md:?}),
            lg: Spacing::px({lg:?}),
            xl: Spacing::px({xl:?}),
        }},
        radius: RadiusScale {{
            sm: Radius::px({rsm:?}),
            md: Radius::px({rmd:?}),
            lg: Radius::px({rlg:?}),
        }},
        typography: TypographyTokens {{
            body: FontSize::px({body:?}),
            heading: FontSize::px({heading:?}),
            mono: FontSize::px({mono:?}),
        }},
    }}
}}
"#,
        name = spec.name,
        b0 = p.background[0],
        b1 = p.background[1],
        b2 = p.background[2],
        b3 = p.background[3],
        s0 = p.surface[0],
        s1 = p.surface[1],
        s2 = p.surface[2],
        s3 = p.surface[3],
        sa0 = p.surface_alt[0],
        sa1 = p.surface_alt[1],
        sa2 = p.surface_alt[2],
        sa3 = p.surface_alt[3],
        tp0 = p.text_primary[0],
        tp1 = p.text_primary[1],
        tp2 = p.text_primary[2],
        tp3 = p.text_primary[3],
        tm0 = p.text_muted[0],
        tm1 = p.text_muted[1],
        tm2 = p.text_muted[2],
        tm3 = p.text_muted[3],
        a0 = p.accent[0],
        a1 = p.accent[1],
        a2 = p.accent[2],
        a3 = p.accent[3],
        ah0 = p.accent_hover[0],
        ah1 = p.accent_hover[1],
        ah2 = p.accent_hover[2],
        ah3 = p.accent_hover[3],
        sel0 = p.selection[0],
        sel1 = p.selection[1],
        sel2 = p.selection[2],
        sel3 = p.selection[3],
        xs = s.xs,
        sm = s.sm,
        md = s.md,
        lg = s.lg,
        xl = s.xl,
        rsm = r.sm,
        rmd = r.md,
        rlg = r.lg,
        body = t.body,
        heading = t.heading,
        mono = t.mono,
    )
}
