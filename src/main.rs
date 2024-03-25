use std::{fs::File, path::PathBuf};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent};

use json_widget::JsonWidget;
use tui::Tui;

mod errors;
mod json_widget;
mod tui;

fn main() -> color_eyre::Result<()> {
    errors::install_hooks()?;
    let tui = tui::init()?;
    let cli = Cli::parse();
    let mut app = JsonEditorApp::new(tui, cli.file_or_default());
    app.run()?;
    tui::restore()?;
    Ok(())
}

#[derive(Debug, Parser)]
struct Cli {
    file: Option<PathBuf>,
}

impl Cli {
    fn file_or_default(&self) -> PathBuf {
        self.file
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demo.json"))
    }
}

struct JsonEditorApp {
    tui: Tui,
    file: PathBuf,
    json: JsonWidget,
}

impl JsonEditorApp {
    fn new(tui: Tui, file: PathBuf) -> Self {
        Self {
            tui,
            file,
            json: JsonWidget::default(),
        }
    }

    fn run(&mut self) -> color_eyre::Result<()> {
        self.json.load(File::open(&self.file)?)?;
        loop {
            self.render()?;
            if let Event::Key(event) = event::read()? {
                if self.handle_key(event)? {
                    break;
                }
            }
        }
        Ok(())
    }

    fn render(&mut self) -> color_eyre::Result<()> {
        self.tui.draw(|frame| {
            frame.render_widget(&self.json, frame.size());
        })?;
        Ok(())
    }

    fn handle_key(&mut self, event: KeyEvent) -> color_eyre::Result<bool> {
        match event.code {
            KeyCode::Char('q') => {
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }
}
