use std::io;

use color_eyre::eyre::Context;
use ratatui::prelude::*;
use serde_json::{Map, Value};

#[derive(Default)]
pub struct JsonWidget {
    json: Value,
}

impl JsonWidget {
    pub fn load<R: io::Read>(&mut self, file: R) -> color_eyre::Result<()> {
        self.json = serde_json::from_reader(file).wrap_err("failed to read file")?;
        Ok(())
    }
}

const PUNCTUATION_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::White);
const KEY_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Blue);
const STRING_STYLE: Style = Style::new().fg(Color::Green);
const NUMBER_STYLE: Style = Style::new().fg(Color::Yellow);
const BOOLEAN_STYLE: Style = Style::new().fg(Color::Cyan);
const NULL_STYLE: Style = Style::new().add_modifier(Modifier::DIM);

impl Widget for &JsonWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // this will be a recursive function that will render the json value
        // to the buffer. It will be called with the root value and the root
        // rect, and will recursively call itself with the subvalues and subrects
        // as it goes down the json tree.
        render_value(&self.json, area, buf);
    }
}

fn render_value(value: &Value, area: Rect, buf: &mut Buffer) {
    let area = area.intersection(buf.area);
    if area.is_empty() {
        return;
    }
    match value {
        Value::Null => render_null(area, buf),
        Value::Bool(b) => render_bool(*b, area, buf),
        Value::Number(num) => render_number(num, area, buf),
        Value::String(s) => render_string(s, area, buf),
        Value::Array(arr) => render_array(&arr, area, buf),
        Value::Object(map) => render_object(&map, area, buf),
    }
}

fn render_null(area: Rect, buf: &mut Buffer) {
    Span::styled("null", NULL_STYLE).render(area, buf);
}

fn render_bool(b: bool, area: Rect, buf: &mut Buffer) {
    let content = if b { "true" } else { "false" };
    Span::styled(content, BOOLEAN_STYLE).render(area, buf);
}

fn render_number(num: &serde_json::Number, area: Rect, buf: &mut Buffer) {
    let content = format!("{}", num);
    Span::styled(content, NUMBER_STYLE).render(area, buf);
}

fn render_string(s: &str, area: Rect, buf: &mut Buffer) {
    // currently single line, but this should be transitioned to multi-line and support wrapping
    let content = format!("\"{}\"", s);
    Span::styled(content, STRING_STYLE).render(area, buf);
}

/// Render an array by rendering the opening bracket on a line by itself, then
/// rendering each value in the array on a separate line, and finally rendering
/// the closing bracket on a line by itself.
///
/// Each value in the array will be rendered indented by 2 spaces.
/// Each value may take up multiple lines, so the area will be split into lines
/// based on the height of the value.
/// The first element of the array will be rendered on the line after the opening
/// bracket, and each subsequent element will be rendered on the line after the
/// previous element.
fn render_array(arr: &[Value], area: Rect, buf: &mut Buffer) {
    let area = area.intersection(buf.area);
    if area.is_empty() {
        return;
    }
    let opening_bracket = Span::styled("[", PUNCTUATION_STYLE);
    let closing_bracket = Span::styled("]", PUNCTUATION_STYLE);
    opening_bracket.render(area, buf);
    if arr.is_empty() {
        // if the array is empty, render the closing bracket on the same line as the opening bracket
        let area = Rect::new(area.left() + 1, area.top(), area.width - 1, 1);
        closing_bracket.render(area, buf);
        return;
    }
    let mut current_line = area.top() + 1;
    for value in arr {
        let value_height = height_of_value(value);
        let value_area = Rect::new(area.left() + 2, current_line, area.width - 2, value_height);
        render_value(value, value_area, buf);
        current_line += value_height;
    }
    let closing_bracket_area = Rect::new(area.left(), current_line, area.width, 1);
    closing_bracket.render(closing_bracket_area, buf);
}

fn render_object(obj: &Map<String, Value>, area: Rect, buf: &mut Buffer) {
    let area = area.intersection(buf.area);
    if area.is_empty() {
        return;
    }
    let opening_brace = Span::styled("{", PUNCTUATION_STYLE);
    let closing_brace = Span::styled("}", PUNCTUATION_STYLE);
    opening_brace.render(area, buf);
    if obj.is_empty() {
        // if the object is empty, render the closing brace on the same line as the opening brace
        let area = Rect::new(area.left() + 1, area.top(), area.width - 1, 1);
        closing_brace.render(area, buf);
        return;
    }
    let mut current_line = area.top() + 1;
    for (key, value) in obj {
        let key_area = Rect::new(area.left() + 2, current_line, area.width - 2, 1);
        let key_span = Span::styled(format!("\"{}\"", key), KEY_STYLE);
        let key_line = Line::from(vec![key_span, Span::styled(": ", PUNCTUATION_STYLE)]);
        let key_width = key_line.width() as u16;
        key_line.render(key_area, buf);
        let value_height = height_of_value(value);
        let value_area = Rect::new(
            area.left() + key_width + 2,
            current_line,
            area.width - key_width - 2,
            value_height,
        );
        render_value(value, value_area, buf);
        current_line += value_height;
    }
    let closing_brace_area = Rect::new(area.left(), current_line, area.width, 1);
    closing_brace.render(closing_brace_area, buf);
}

fn height_of_value(value: &Value) -> u16 {
    match value {
        Value::Null => 1,
        Value::Bool(_) => 1,
        Value::Number(_) => 1,
        Value::String(s) => s.lines().count() as u16,
        Value::Array(arr) => {
            if arr.is_empty() {
                1
            } else {
                arr.iter().map(height_of_value).sum::<u16>() + 2
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                1
            } else {
                map.values().map(height_of_value).sum::<u16>() + 2
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};
    use serde_json::{json, Number};

    use super::*;

    /// A 10x1 buffer
    #[fixture]
    fn small_buf() -> Buffer {
        Buffer::empty(Rect::new(0, 0, 10, 1))
    }

    /// A 20x5 buffer
    #[fixture]
    fn medium_buf() -> Buffer {
        Buffer::empty(Rect::new(0, 0, 20, 5))
    }

    /// A 20x10 buffer
    #[fixture]
    fn large_buf() -> Buffer {
        Buffer::empty(Rect::new(0, 0, 20, 10))
    }

    #[rstest]
    #[case(json!(null), "null      ")]
    #[case(json!(true), "true      ")]
    #[case(json!(false), "false     ")]
    #[case(json!(42), "42        ")]
    #[case(json!(-1), "-1        ")]
    #[case(json!(42.0), "42.0      ")]
    #[case(json!(-1.0), "-1.0      ")]
    #[case(json!("Hello, World!"), "Hello, Wor")]
    fn render_value_null(mut small_buf: Buffer, #[case] value: Value, #[case] expected: &str) {
        render_value(&value, small_buf.area, &mut small_buf);
        assert_eq!(small_buf, Buffer::with_lines(vec![expected]));
    }

    #[rstest]
    fn render_null(mut small_buf: Buffer) {
        super::render_null(small_buf.area, &mut small_buf);
        assert_eq!(small_buf, Buffer::with_lines(vec!["null      "]));
    }

    #[rstest]
    fn render_bool(mut small_buf: Buffer) {
        super::render_bool(true, small_buf.area, &mut small_buf);
        assert_eq!(small_buf, Buffer::with_lines(vec!["true      "]));
    }

    #[rstest]
    fn render_number(mut small_buf: Buffer) {
        let num = Number::from_f64(42.0).unwrap();
        super::render_number(&num, small_buf.area, &mut small_buf);
        assert_eq!(small_buf, Buffer::with_lines(vec!["42.0      "]));
    }

    #[rstest]
    fn render_string(mut small_buf: Buffer) {
        super::render_string("Hello, World!", small_buf.area, &mut small_buf);
        assert_eq!(small_buf, Buffer::with_lines(vec!["Hello, Wor"]));
    }

    #[rstest]
    fn render_array_empty(mut small_buf: Buffer) {
        let arr = vec![];
        super::render_array(&arr, small_buf.area, &mut small_buf);
        assert_eq!(small_buf, Buffer::with_lines(vec!["[]        "]));
    }

    #[rstest]
    fn render_array(mut medium_buf: Buffer) {
        let arr = vec![json!(null), json!(true), json!(42)];
        super::render_array(&arr, medium_buf.area, &mut medium_buf);
        assert_eq!(
            medium_buf,
            Buffer::with_lines(vec![
                "[                   ",
                "  null              ",
                "  true              ",
                "  42                ",
                "]                   ",
            ])
        );
    }

    #[rstest]
    fn render_array_nested(mut medium_buf: Buffer) {
        let arr = vec![json!([42])];
        super::render_array(&arr, medium_buf.area, &mut medium_buf);
        assert_eq!(
            medium_buf,
            Buffer::with_lines(vec![
                "[                   ",
                "  [                 ",
                "    42              ",
                "  ]                 ",
                "]                   ",
            ])
        );
    }

    #[rstest]
    fn render_object_empty(mut small_buf: Buffer) {
        let obj = Map::new();
        super::render_object(&obj, small_buf.area, &mut small_buf);
        assert_eq!(small_buf, Buffer::with_lines(vec!["{}        "]));
    }

    #[rstest]
    fn render_object(mut medium_buf: Buffer) {
        let mut obj = Map::default();
        obj.insert("key1".into(), json!(42));
        obj.insert("key2".into(), json!(true));
        super::render_object(&obj, medium_buf.area, &mut medium_buf);
        assert_eq!(
            medium_buf,
            Buffer::with_lines(vec![
                "{                   ",
                "  key1: 42          ",
                "  key2: true        ",
                "}                   ",
                "                    ",
            ])
        );
    }

    #[rstest]
    #[case(json!(null), 1)]
    #[case(json!(true), 1)]
    #[case(json!(42), 1)]
    #[case(json!(-1), 1)]
    #[case(json!(42.0), 1)]
    #[case(json!(-1.0), 1)]
    #[case(json!("Hello, World!"), 1)]
    // empty array is 1 line
    #[case(json!([]), 1)]
    // other arrays always have a line for the opening bracket, a line for the closing bracket,
    // and a line for each element
    #[case(json!([null]), 3)]
    #[case(json!([true]), 3)]
    #[case(json!([42]), 3)]
    #[case(json!([42, 42]), 4)]
    #[case(json!([42, 42, 42]), 5)]
    #[case(json!([[]]), 3)]
    #[case(json!([[42]]), 5)]
    #[case(json!([[42], [42]]), 8)]
    #[case(json!({}), 1)]
    #[case(json!({"key": null}), 3)]
    #[case(json!({"key": true}), 3)]
    #[case(json!({"key": 42}), 3)]
    #[case(json!({"key": []}), 3)]
    #[case(json!({"key1": 42, "key2": 42}), 4)]
    #[case(json!({"key1": 42, "key2": []}), 4)]
    #[case(json!({"key1": 42, "key2": [42]}), 6)]
    #[case(json!({"key1": 42, "key2": [42, 42]}), 7)]
    #[case(json!({"key1": 42, "key2": [42, 42, 42]}), 8)]
    #[case(json!({"key1": 42, "key2": {"key3": 42}}), 6)]
    fn height_of_value(#[case] value: Value, #[case] expected: u16) {
        assert_eq!(
            super::height_of_value(&value),
            expected,
            "value: {:?}",
            value
        );
    }
}
