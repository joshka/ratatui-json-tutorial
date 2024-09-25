use std::io;

use color_eyre::eyre::Context;
use itertools::{Itertools, Position};
use ratatui::{prelude::*, widgets::Paragraph};
use serde_json::{Map, Number, Value};

#[derive(Default)]
pub struct JsonWidget {
    style: JsonWidgetStyle,
    json: Value,
    edit_index: usize,
    show_debug: bool,
}

impl JsonWidget {
    pub fn new(value: Value) -> Self {
        Self {
            style: JsonWidgetStyle::default(),
            json: value,
            edit_index: 0,
            show_debug: false,
        }
    }

    pub fn load<R: io::Read>(&mut self, reader: R) -> color_eyre::Result<()> {
        self.json = serde_json::from_reader(reader).wrap_err("failed to read file")?;
        Ok(())
    }

    pub fn next_edit(&mut self) {
        self.edit_index = self.edit_index.saturating_add(1);
    }

    pub fn prev_edit(&mut self) {
        self.edit_index = self.edit_index.saturating_sub(1);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct JsonWidgetStyle {
    pub punctuation: Style,
    pub key: Style,
    pub string: Style,
    pub number: Style,
    pub boolean: Style,
    pub null: Style,
}

impl Default for JsonWidgetStyle {
    fn default() -> Self {
        Self {
            punctuation: (Color::White, Modifier::BOLD).into(),
            key: (Color::Blue, Modifier::BOLD).into(),
            string: Color::Green.into(),
            number: Color::Yellow.into(),
            boolean: Color::Cyan.into(),
            null: (Color::White, Modifier::DIM).into(),
        }
    }
}

// TODO make this a stateful widget as the edit index needs to be mutated based on the number of
// items in the json
impl Widget for &JsonWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let debug_width = bool::from(self.show_debug) as u16; // 0 or 1
        let [left, right] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(debug_width)]).areas(area);
        let mut visitor = TextVisitor::new(self.style);
        visitor.visit_value(&self.json);
        let debug = format!("{:#?}", visitor.edit_positions);
        Text::raw(debug).render(right, buf);

        if let Some(span) = visitor.get_span_mut(self.edit_index) {
            span.style = span.style.add_modifier(Modifier::REVERSED);
        }
        let index = self
            .edit_index
            .clamp(0, visitor.edit_positions.len().saturating_sub(1));
        let y = visitor
            .edit_positions
            .get(index)
            .map(|pos| pos.line_index as u16)
            .unwrap_or(0)
            .saturating_sub(left.height - 2)
            .clamp(0, left.height);
        Paragraph::new(visitor.text)
            .scroll((y, 0))
            .render(left, buf);
    }
}

trait Visit {
    fn visit_value(&mut self, value: &Value) {
        match value {
            Value::Null => self.visit_null(),
            Value::Bool(b) => self.visit_bool(*b),
            Value::Number(num) => self.visit_number(num),
            Value::String(s) => self.visit_string(s),
            Value::Array(arr) => self.visit_array(arr),
            Value::Object(map) => self.visit_object(map),
        }
    }
    fn visit_null(&mut self);
    fn visit_bool(&mut self, b: bool);
    fn visit_number(&mut self, num: &Number);
    fn visit_string(&mut self, s: &str);
    fn visit_array(&mut self, arr: &[Value]);
    fn visit_object(&mut self, map: &Map<String, Value>);
    fn visit_key_value(&mut self, key: &str, value: &Value);
    fn visit_key(&mut self, key: &str);
}

#[derive(Debug)]
struct TextVisitor<'a> {
    style: JsonWidgetStyle,
    indent: usize,
    pub text: Text<'a>,
    pub edit_positions: Vec<EditPosition>,
}

#[derive(Debug, Clone, Copy)]
pub struct EditPosition {
    line_index: usize,
    span_index: usize,
}

impl<'a> TextVisitor<'a> {
    fn new(style: JsonWidgetStyle) -> Self {
        Self {
            style,
            text: Text::raw(""),
            indent: 0,
            edit_positions: Vec::new(),
        }
    }

    fn incr_indent(&mut self) {
        self.indent += 2;
    }

    fn decr_indent(&mut self) {
        self.indent -= 2;
    }

    fn push_line(&mut self) {
        self.text.push_line(" ".repeat(self.indent))
    }

    fn push_value<S: ToString>(&mut self, value: S, style: Style) {
        let span = Span::styled(value.to_string(), style);
        self.text.push_span(span);
        self.push_edit_position();
    }

    fn push_key(&mut self, key: &str) {
        let span = Span::styled(format!("\"{}\"", key), self.style.key);
        self.text.push_span(span);
        self.push_edit_position();
    }

    fn push_punctuation(&mut self, punctuation: &'static str) {
        let span = Span::styled(punctuation, self.style.punctuation);
        self.text.push_span(span);
    }

    fn push_edit_position(&mut self) {
        self.edit_positions.push(EditPosition {
            line_index: self.text.lines.len() - 1,
            span_index: self.text.lines.last().unwrap().spans.len() - 1,
        });
    }

    fn get_span_mut(&mut self, index: usize) -> Option<&mut Span<'a>> {
        let position = self.edit_positions.get(index)?;
        let line = self.text.lines.get_mut(position.line_index)?;
        let span = line.spans.get_mut(position.span_index)?;
        Some(span)
    }
}

impl Visit for TextVisitor<'_> {
    fn visit_null(&mut self) {
        self.push_value("null", self.style.null);
    }

    fn visit_bool(&mut self, b: bool) {
        self.push_value(b, self.style.boolean);
    }

    fn visit_number(&mut self, num: &Number) {
        self.push_value(num, self.style.number);
    }

    fn visit_string(&mut self, s: &str) {
        self.push_value(format!("\"{}\"", s), self.style.string);
    }

    fn visit_array(&mut self, arr: &[Value]) {
        self.push_punctuation("[");
        self.incr_indent();
        for (position, value) in arr.iter().with_position() {
            if position == Position::First {
                self.push_punctuation(", ");
            }
            self.push_line();
            self.visit_value(value);
        }
        self.decr_indent();
        if !arr.is_empty() {
            self.push_line();
        }
        self.push_punctuation("]");
    }

    fn visit_object(&mut self, map: &Map<String, Value>) {
        self.push_punctuation("{");
        self.incr_indent();
        for (position, (key, value)) in map.iter().with_position() {
            if position != Position::First {
                self.push_punctuation(", ");
            }
            self.visit_key_value(key, value);
        }
        self.decr_indent();
        // only add a newline if there are any key-value pairs in the object
        if !map.is_empty() {
            self.push_line();
        }
        self.push_punctuation("}");
    }

    fn visit_key_value(&mut self, key: &str, value: &Value) {
        self.push_line();
        self.visit_key(key);
        self.push_punctuation(": ");
        self.visit_value(value);
    }

    fn visit_key(&mut self, key: &str) {
        self.push_key(key);
    }
}
