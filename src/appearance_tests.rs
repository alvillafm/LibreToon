use crate::{
    i18n::{self, Language},
    studio::Preferences,
    themes::{self, Theme},
};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
fn contrast(a: Color32, b: Color32) -> f32 {
    let luminance = |c: Color32| {
        let linear = |v: u8| {
            let f = v as f32 / 255.;
            if f <= 0.04045 {
                f / 12.92
            } else {
                ((f + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
    };
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
#[test]
fn original_dark_is_default_and_every_theme_has_readable_text() {
    let prefs = Preferences::default();
    assert_eq!(prefs.theme, Theme::Studio);
    let ctx = egui::Context::default();
    for theme in Theme::ALL {
        themes::ensure(&ctx, theme);
        let p = theme.palette();
        assert!(contrast(p.text, p.panel) >= 4.5, "{theme:?}");
        assert!(contrast(p.muted, p.panel) >= 4.5, "muted {theme:?}");
        assert_eq!(
            ctx.style_of(if p.light {
                egui::Theme::Light
            } else {
                egui::Theme::Dark
            })
            .visuals
            .panel_fill,
            p.panel
        );
    }
    themes::ensure(&ctx, Theme::Studio);
    assert_eq!(themes::color(Color32::WHITE), Color32::WHITE);
    assert_eq!(
        ctx.style_of(egui::Theme::Dark).visuals.panel_fill,
        crate::design::PANEL
    );
}
#[test]
fn locale_choices_have_complete_catalogues_and_preserve_placeholders() {
    let baseline = i18n::catalogue(Language::English);
    let params = |text: &str| {
        let mut values: Vec<String> = text
            .split('{')
            .skip(1)
            .filter_map(|p| p.split_once('}').map(|(p, _)| p.to_owned()))
            .collect();
        values.sort();
        values
    };
    for language in Language::ALL {
        let catalog = i18n::catalogue(language);
        assert_eq!(catalog.len(), baseline.len(), "{language:?}");
        for key in baseline.keys() {
            let value = catalog
                .get(key)
                .unwrap_or_else(|| panic!("missing {language:?}: {key}"));
            assert!(!value.is_empty(), "{language:?}: {key}");
            assert_eq!(params(key), params(value), "{language:?}: {key}");
        }
    }
}
#[test]
fn locales_switch_immediately_and_portuguese_regions_are_distinct() {
    for (language, expected) in [
        (Language::English, "Save"),
        (Language::Spanish, "Guardar"),
        (Language::German, "Speichern"),
        (Language::French, "Enregistrer"),
        (Language::Italian, "Salva"),
        (Language::PortugueseBrazil, "Salvar"),
        (Language::PortuguesePortugal, "Guardar"),
        (Language::Russian, "Сохранить"),
        (Language::ChineseSimplified, "保存"),
        (Language::Japanese, "保存"),
        (Language::Korean, "저장"),
    ] {
        i18n::set_locale(language);
        assert_eq!(i18n::tr("Guardar"), expected);
    }
    i18n::set_locale(Language::English);
    let message = i18n::message(
        "«{name}» tiene cambios sin guardar.",
        &[("name", "动画.fla".into())],
    );
    assert!(message.contains("动画.fla"));
    assert!(!message.contains("{name}"));
}
#[test]
fn preferences_keep_theme_locale_shortcuts_and_legacy_spanish() {
    let mut prefs: Preferences = serde_json::from_str(r#"{"spanish":true,"dpi":144}"#).unwrap();
    assert_eq!(prefs.locale(), Language::Spanish);
    assert_eq!(prefs.theme, Theme::Studio);
    prefs.language = Some(prefs.locale());
    prefs.theme = Theme::Paper;
    let copy: Preferences = serde_json::from_slice(&serde_json::to_vec(&prefs).unwrap()).unwrap();
    assert_eq!(copy.locale(), Language::Spanish);
    assert_eq!(copy.theme, Theme::Paper);
    assert_eq!(copy.dpi, 144.);
    assert!(copy.bindings == prefs.bindings);
    let future: Preferences =
        serde_json::from_str(r#"{"language":"future-language","dpi":120}"#).unwrap();
    assert_eq!(future.locale(), Language::English);
    assert_eq!(future.dpi, 120.);
}
#[test]
fn bundled_font_covers_locale_names_cyrillic_and_cjk() {
    let data = include_bytes!("../assets/fonts/NotoSansCJK-Regular.ttc");
    for (index, text) in [
        (0, "Русский 日本語 ひらがな カタカナ"),
        (1, "한국어 한글"),
        (2, "简体中文 动画 帧 权重"),
    ] {
        let face = ttf_parser::Face::parse(data, index).unwrap();
        for c in text.chars().filter(|c| !c.is_whitespace()) {
            assert!(face.glyph_index(c).is_some(), "missing {c} in face {index}");
        }
    }
}
#[test]
fn all_themes_and_asian_locales_render_without_changing_artwork() {
    let ctx = egui::Context::default();
    let mut e = crate::Editor::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
    e.recovery = std::env::temp_dir().join(format!(
        "appearance-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ));
    e.add_document(crate::fresh_document(), None);
    e.doc.layers[0].frames[0].push(crate::document::Mark {
        points: vec![[40., 50.], [180., 160.]].into(),
        rectangle: true,
        closed: true,
        fill: Some([220, 40, 130, 255]),
        ..Default::default()
    });
    let before = serde_json::to_vec(&e.doc).unwrap();
    let color = e.color;
    let fill = e.fill_color;
    for theme in Theme::ALL {
        for language in [
            Language::English,
            Language::Japanese,
            Language::ChineseSimplified,
            Language::Korean,
        ] {
            e.studio.prefs.theme = theme;
            e.studio.prefs.language = Some(language);
            themes::ensure(&ctx, theme);
            i18n::set_locale(language);
            i18n::ensure_fonts(&ctx, language);
            for _ in 0..2 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1500., 1000.))),
                        ..Default::default()
                    },
                    |ui| {
                        e.workspace(ui);
                        egui::CentralPanel::default().show(ui, |ui| e.canvas_ui(ui));
                    },
                );
                assert!(!output.shapes.is_empty());
                output.textures_delta.clear();
            }
            assert_eq!(serde_json::to_vec(&e.doc).unwrap(), before);
            assert_eq!(e.color, color);
            assert_eq!(e.fill_color, fill);
        }
    }
    i18n::set_locale(Language::English);
    themes::ensure(&ctx, Theme::Studio);
}
