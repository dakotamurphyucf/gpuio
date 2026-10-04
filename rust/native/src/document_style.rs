//! Resolve validated rich-reader presentation once per configuration/theme change.
use gpui::{Refineable, px, rems};
use gpuio_protocol::{
    document_style::{Config, Part},
    v1::*,
};

pub fn validate(config: &Config) -> Result<(), ErrorCode> {
    if !config.is_valid() {
        return Err(ErrorCode::Malformed);
    }
    for styles in config.styles() {
        for style in styles {
            let Style::Fields(fields) = style else {
                unreachable!("validated part styles");
            };
            crate::style::validate_fields(fields)?;
        }
    }
    Ok(())
}
pub fn retained_bytes(config: &Config) -> usize {
    std::mem::size_of::<Config>()
        + config.colors.len() * std::mem::size_of::<(Part, Color)>()
        + config
            .styles()
            .into_iter()
            .map(|styles| {
                std::mem::size_of_val(styles)
                    + styles
                        .iter()
                        .map(crate::style::retained_bytes)
                        .sum::<usize>()
            })
            .sum::<usize>()
}
fn color(color: &Color) -> gpui::Hsla {
    let Color::Rgba(n) = color else {
        unreachable!("validated document color");
    };
    gpui::rgba(*n as u32).into()
}
fn refinement(base: &gpui::StyleRefinement, styles: &[Style]) -> gpui::StyleRefinement {
    let mut out = base.clone();
    out.refine(&crate::appearance::refinement(styles, 0));
    out
}
pub fn resolve(mut style: gpui_base::TextViewStyle, config: &Config) -> gpui_base::TextViewStyle {
    for (part, value) in &config.colors {
        let c = color(value);
        style = match part {
            Part::Foreground => style.with_foreground(c),
            Part::MutedForeground => style.with_muted_foreground(c),
            Part::Link => style.with_link(c),
            Part::Selection => style.with_selection(c),
            Part::CodeBackground => style.with_code_background(c),
            Part::Border => style.with_border(c),
        };
    }
    if let Some(gap) = config.paragraph_gap_rem {
        style = style.with_paragraph_gap(rems(gap as f32));
    }
    if let Some(size) = config.heading_base_font_size {
        style = style.with_heading_base_font_size(px(size as f32));
    }
    if let Some(sizes) = &config.heading_sizes {
        let sizes = sizes.values();
        style = style.with_heading_font_size(move |level, _| {
            px(sizes[usize::from(level.clamp(1, 6) - 1)] as f32)
        });
    }
    let code = &config.inline_code;
    style = style.with_inline_code(gpui::HighlightStyle {
        color: code.foreground.as_ref().map(color),
        background_color: code.background.as_ref().map(color),
        font_weight: code.font_weight.map(|n| gpui::FontWeight(n as f32)),
        font_style: code.italic.map(|italic| {
            if italic {
                gpui::FontStyle::Italic
            } else {
                gpui::FontStyle::Normal
            }
        }),
        underline: code.underline.as_ref().map(|u| gpui::UnderlineStyle {
            color: u.color.as_ref().map(color),
            thickness: px(u.thickness as f32),
            wavy: u.wavy,
        }),
        strikethrough: code
            .strikethrough
            .as_ref()
            .map(|u| gpui::StrikethroughStyle {
                color: u.color.as_ref().map(color),
                thickness: px(u.thickness as f32),
            }),
        fade_out: code.fade_out.map(|n| n as f32),
    });
    let code_block = refinement(style.code_block(), &config.code_block);
    let table = refinement(style.table(), &config.table);
    let table_head = refinement(style.table_head(), &config.table_head);
    let table_cell = refinement(style.table_cell(), &config.table_cell);
    style
        .with_code_block(code_block)
        .with_table(table)
        .with_table_head(table_head)
        .with_table_cell(table_cell)
}
