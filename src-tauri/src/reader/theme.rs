// mercury4win-linux/src-tauri/reader/theme.rs
// Theme CSS generation for reader content (classic/paper, light/dark)

use crate::db::models::Content;

/// Wrap rendered HTML body with article wrapper and theme CSS.
pub fn wrap_with_theme(body_html: &str, theme_id: &str) -> String {
    let parts: Vec<&str> = theme_id.split('-').collect();
    let (style, color_scheme) = match parts.as_slice() {
        [style, color] => (*style, *color),
        _ => ("classic", "light"),
    };

    let css = css_for_theme(style, color_scheme);

    format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><style>{css}</style></head>
<body>
<article class="reader">
{body_html}
</article>
</body>
</html>"#,
    )
}

fn css_for_theme(style: &str, color_scheme: &str) -> String {
    let (bg, text, muted, accent) = match color_scheme {
        "dark" => ("#1a1a2e", "#e0e0e0", "#888", "#6ca0ff"),
        _ => ("#ffffff", "#1a1a1a", "#666", "#2563eb"),
    };

    let font = match style {
        "paper" => "\"Georgia\", \"Times New Roman\", serif",
        _ => "\"Inter\", -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
    };

    format!(
        r#"
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{
    background: {bg};
    color: {text};
    font-family: {font_family};
    line-height: 1.8;
    padding: 2rem 1.5rem;
    max-width: 720px;
    margin: 0 auto;
}}
h1 {{ font-size: 2rem; font-weight: 700; margin: 2rem 0 1rem; }}
h2 {{ font-size: 1.5rem; font-weight: 600; margin: 1.5rem 0 0.75rem; }}
h3 {{ font-size: 1.25rem; font-weight: 600; margin: 1.25rem 0 0.5rem; }}
h4 {{ font-size: 1.1rem; font-weight: 600; margin: 1rem 0 0.5rem; }}
p {{ margin: 0.75rem 0; }}
a {{ color: {accent}; text-decoration: underline; }}
img {{ max-width: 100%; border-radius: 0.5rem; margin: 1rem 0; }}
blockquote {{
    border-left: 4px solid {accent};
    padding-left: 1rem;
    margin: 1rem 0;
    color: {muted};
    font-style: italic;
}}
code {{
    background: {code_bg};
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-family: {mono_font};
    font-size: 0.9em;
}}
pre {{
    background: {code_bg};
    padding: 1rem;
    border-radius: 0.5rem;
    overflow-x: auto;
    margin: 1rem 0;
}}
pre code {{ background: none; padding: 0; }}
ul, ol {{ margin: 0.75rem 0; padding-left: 1.5rem; }}
li {{ margin: 0.25rem 0; }}
table {{ width: 100%; border-collapse: collapse; margin: 1rem 0; }}
thead {{ background: {code_bg}; }}
th, td {{ border: 1px solid {border_color}; padding: 0.5rem 0.75rem; text-align: left; }}
th {{ font-weight: 600; font-size: 0.9em; }}
del {{ text-decoration: line-through; color: {muted}; }}
hr {{ border: none; border-top: 1px solid {border_color}; margin: 2rem 0; }}
figure {{ margin: 1.5rem 0; }}
figcaption {{ font-size: 0.9em; color: {muted}; margin-top: 0.5rem; }}
"#,
        bg = bg,
        text = text,
        font_family = font,
        accent = accent,
        muted = muted,
        code_bg = if color_scheme == "dark" { "#2a2a3e" } else { "#f3f4f6" },
        mono_font = "\"JetBrains Mono\", ui-monospace, monospace",
        border_color = if color_scheme == "dark" { "#333" } else { "#e5e7eb" },
    )
}

/// Get the appropriate theme_id for a given style and color scheme.
pub fn theme_id(style: &str, color_scheme: &str) -> String {
    format!("{}-{}", style, color_scheme)
}

/// Default theme IDs available.
pub const DEFAULT_THEMES: &[(&str, &str)] = &[
    ("classic", "light"),
    ("classic", "dark"),
    ("paper", "light"),
    ("paper", "dark"),
];
