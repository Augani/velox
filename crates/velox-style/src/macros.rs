#[macro_export]
macro_rules! theme {
    (
        name: $name:expr,
        palette: {
            background: [$bg_r:expr, $bg_g:expr, $bg_b:expr, $bg_a:expr],
            surface: [$sf_r:expr, $sf_g:expr, $sf_b:expr, $sf_a:expr],
            surface_alt: [$sa_r:expr, $sa_g:expr, $sa_b:expr, $sa_a:expr],
            text_primary: [$tp_r:expr, $tp_g:expr, $tp_b:expr, $tp_a:expr],
            text_muted: [$tm_r:expr, $tm_g:expr, $tm_b:expr, $tm_a:expr],
            accent: [$ac_r:expr, $ac_g:expr, $ac_b:expr, $ac_a:expr],
            accent_hover: [$ah_r:expr, $ah_g:expr, $ah_b:expr, $ah_a:expr],
            selection: [$se_r:expr, $se_g:expr, $se_b:expr, $se_a:expr] $(,)?
        },
        space: {
            xs: $xs:expr,
            sm: $sm:expr,
            md: $md:expr,
            lg: $lg:expr,
            xl: $xl:expr $(,)?
        },
        radius: {
            sm: $rsm:expr,
            md: $rmd:expr,
            lg: $rlg:expr $(,)?
        },
        typography: {
            body: $body:expr,
            heading: $heading:expr,
            mono: $mono:expr $(,)?
        } $(,)?
    ) => {
        $crate::Theme {
            name: $name,
            palette: $crate::Palette {
                background: $crate::ThemeColor::rgba($bg_r, $bg_g, $bg_b, $bg_a),
                surface: $crate::ThemeColor::rgba($sf_r, $sf_g, $sf_b, $sf_a),
                surface_alt: $crate::ThemeColor::rgba($sa_r, $sa_g, $sa_b, $sa_a),
                text_primary: $crate::ThemeColor::rgba($tp_r, $tp_g, $tp_b, $tp_a),
                text_muted: $crate::ThemeColor::rgba($tm_r, $tm_g, $tm_b, $tm_a),
                accent: $crate::ThemeColor::rgba($ac_r, $ac_g, $ac_b, $ac_a),
                accent_hover: $crate::ThemeColor::rgba($ah_r, $ah_g, $ah_b, $ah_a),
                selection: $crate::ThemeColor::rgba($se_r, $se_g, $se_b, $se_a),
            },
            space: $crate::SpaceScale {
                xs: $crate::Spacing::px($xs),
                sm: $crate::Spacing::px($sm),
                md: $crate::Spacing::px($md),
                lg: $crate::Spacing::px($lg),
                xl: $crate::Spacing::px($xl),
            },
            radius: $crate::RadiusScale {
                sm: $crate::Radius::px($rsm),
                md: $crate::Radius::px($rmd),
                lg: $crate::Radius::px($rlg),
            },
            typography: $crate::TypographyTokens {
                body: $crate::FontSize::px($body),
                heading: $crate::FontSize::px($heading),
                mono: $crate::FontSize::px($mono),
            },
        }
    };
}
