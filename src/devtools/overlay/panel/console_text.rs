//! ANSI-free copy text and bounded SGR coloring for console rows.
use super::*;

pub(super) fn console_plain_text(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for control in chars.by_ref() {
                if ('@'..='~').contains(&control) {
                    break;
                }
            }
        } else if ch != '\u{1b}' {
            plain.push(ch);
        }
    }
    plain
}

#[derive(Clone, Copy, Default)]
struct AnsiStyle {
    color: Option<[f32; 3]>,
    bold: bool,
}

pub(super) fn draw_ansi_text(ui: &imgui::Ui, text: &str, severity_color: [f32; 4]) {
    let mut style = AnsiStyle::default();
    let mut plain = String::new();
    let mut spans = 0;
    let mut has_drawn_span = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' || chars.peek() != Some(&'[') {
            if ch != '\u{1b}' {
                plain.push(ch);
            }
            continue;
        }
        chars.next();
        let mut params = String::new();
        let mut valid = true;
        let mut terminated = false;
        for control in chars.by_ref().take(32) {
            if control == 'm' {
                terminated = true;
                break;
            }
            if !control.is_ascii_digit() && control != ';' {
                valid = false;
                break;
            }
            params.push(control);
        }
        if !valid || !terminated || spans >= crate::api::scripts::console::MAX_ANSI_SGR_SPANS {
            continue;
        }
        let previous = style;
        if apply_sgr(&params, &mut style) {
            if !plain.is_empty() {
                draw_ansi_span(ui, &plain, previous, severity_color, &mut has_drawn_span);
                plain.clear();
            }
            spans += 1;
        }
    }
    if !plain.is_empty() {
        draw_ansi_span(ui, &plain, style, severity_color, &mut has_drawn_span);
    }
}

fn apply_sgr(params: &str, style: &mut AnsiStyle) -> bool {
    let codes = if params.is_empty() {
        vec![0]
    } else {
        let Ok(codes) = params
            .split(';')
            .map(str::parse::<u8>)
            .collect::<Result<Vec<_>, _>>()
        else {
            return false;
        };
        codes
    };
    if codes
        .iter()
        .any(|code| !matches!(*code, 0 | 1 | 30..=37 | 90..=97))
    {
        return false;
    }
    for code in codes {
        match code {
            0 => *style = AnsiStyle::default(),
            1 => style.bold = true,
            30..=37 => style.color = Some(ansi_color(code - 30, false)),
            90..=97 => style.color = Some(ansi_color(code - 90, true)),
            _ => unreachable!(),
        }
    }
    true
}

pub(super) fn ansi_color(index: u8, bright: bool) -> [f32; 3] {
    const NORMAL: [[f32; 3]; 8] = [
        [0.0, 0.0, 0.0],
        [0.8, 0.0, 0.0],
        [0.0, 0.65, 0.0],
        [0.8, 0.55, 0.0],
        [0.0, 0.0, 0.8],
        [0.7, 0.0, 0.7],
        [0.0, 0.65, 0.65],
        [0.8, 0.8, 0.8],
    ];
    const BRIGHT: [[f32; 3]; 8] = [
        [0.45, 0.45, 0.45],
        [1.0, 0.35, 0.35],
        [0.35, 1.0, 0.35],
        [1.0, 0.9, 0.35],
        [0.4, 0.55, 1.0],
        [1.0, 0.4, 1.0],
        [0.4, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    ];
    if bright {
        BRIGHT[index as usize]
    } else {
        NORMAL[index as usize]
    }
}

fn draw_ansi_span(
    ui: &imgui::Ui,
    text: &str,
    style: AnsiStyle,
    severity: [f32; 4],
    has_drawn_span: &mut bool,
) {
    if *has_drawn_span {
        ui.same_line();
    }
    let mut color = style
        .color
        .unwrap_or([severity[0], severity[1], severity[2]]);
    if style.bold {
        for component in &mut color {
            *component = (*component * 1.2).min(1.0);
        }
    }
    let _color = ui.push_style_color(
        imgui::StyleColor::Text,
        [color[0], color[1], color[2], severity[3]],
    );
    ui.text_colored([color[0], color[1], color[2], severity[3]], text);
    *has_drawn_span = true;
}

#[cfg(test)]
mod console_text_tests {
    use super::console_plain_text;

    #[test]
    fn copied_console_text_does_not_include_ansi_color_sequences() {
        assert_eq!(
            console_plain_text("\u{1b}[1;92mSCRIPT: test\u{1b}[0m ready"),
            "SCRIPT: test ready"
        );
    }
}
