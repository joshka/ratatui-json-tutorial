use std::{fs::File, path::PathBuf};

use clap::Parser;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};

use json_widget::JsonWidget;
use ratatui::{
    layout::{Constraint, Layout},
    style::Stylize,
    text::Line,
    DefaultTerminal,
};

mod json_widget;

fn main() -> Result<()> {
    color_eyre::install()?;
    let tui = ratatui::init();
    let cli = Cli::parse();
    let mut app = JsonEditorApp::new(tui, cli.file);
    let result = app.run();
    ratatui::restore();
    result
}

#[derive(Debug, Parser)]
struct Cli {
    #[arg(default_value = "demo.json")]
    file: PathBuf,
}

struct JsonEditorApp {
    quit: bool,
    tui: DefaultTerminal,
    file: PathBuf,
    json: JsonWidget,
}

impl JsonEditorApp {
    fn new(tui: DefaultTerminal, file: PathBuf) -> Self {
        Self {
            quit: false,
            tui,
            file,
            json: JsonWidget::default(),
        }
    }

    fn run(&mut self) -> Result<()> {
        let reader = File::open(&self.file)?;
        self.json.load(reader)?;
        while !self.quit {
            self.draw()?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        self.tui.draw(|frame| {
            let [title, main] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(frame.area());
            let line = Line::from("JSON editor tutorial example. [k prev] [j next] [q quit]")
                .white()
                .on_blue();
            frame.render_widget(line, title);
            frame.render_widget(&self.json, main);
        })?;
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(event) = event::read()? {
            self.handle_key(event);
        }
        Ok(())
    }

    fn handle_key(&mut self, event: KeyEvent) {
        use KeyCode::*;
        match event.code {
            Char('q') | Esc => self.quit = true,
            Char('j') | Char('l') | Down | Right => self.json.next_edit(),
            Char('k') | Char('h') | Up | Left => self.json.prev_edit(),
            _ => {}
        }
    }
}
