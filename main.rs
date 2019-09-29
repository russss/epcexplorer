mod util;

extern crate failure;
extern crate gs1;
extern crate hex;
extern crate ru5102;
extern crate termion;
extern crate tui;

mod app;
mod rfid;
mod tagtable;
mod tagdetail;

use std::env;
use std::io;
use std::panic;
use std::sync::mpsc;
use std::thread;
use std::process::exit;

use crate::app::App;
use crate::rfid::scan_thread;
use crate::tagtable::TagTable;
use crate::tagdetail::TagDetail;

use failure::bail;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Layout};
use tui::style::{Style, Color};
use tui::widgets::{Widget, Block, Borders};
use tui::Terminal;

use crate::util::event::{Event, Events};

fn main() -> Result<(), failure::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: epcexplorer <serial device>");
        exit(1);
    }

    let reader = match ru5102::Reader::new(&args[1]) {
        Ok(res) => res,
        Err(e) => {
            bail!(format!(
                "Unable to connect to reader {:?}: {:?}",
                args[1], e
            ));
        }
    };

    panic::set_hook(Box::new(panic_hook));

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    let mut app = App::new();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        scan_thread(reader, tx);
    });

    loop {
        terminal.draw(|mut f| {
            let items = app.get_items();
            let selected = app.selected.to_owned();
            let selected_item = match selected {
                Some(epc) => {
                    app.items.get(&epc)  
                },
                None => None
            };
            let rects = Layout::default()
                .constraints([
                    Constraint::Percentage(80),
                    Constraint::Percentage(20),
                ].as_ref())
                .split(f.size());
            TagTable::new(&items, app.selected.to_owned()).render(&mut f, rects[0]);
            TagDetail::new(selected_item).render(&mut f, rects[1]);
        })?;

        match events.next()? {
            Event::Input(key) => match key {
                Key::Char('q') => {
                    break;
                }
                Key::Down => {
                    app.update_selected(false);
                }
                Key::Up => {
                    app.update_selected(true);
                }
                _ => {}
            },
            Event::Tick => {
                app.update_items(&rx);
            }
        };
    }

    Ok(())
}

pub(crate) fn block<'a>(title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default().fg(Color::Red)
        )
}

fn panic_hook(info: &panic::PanicInfo<'_>) {
    let location = info.location().unwrap(); // The current implementation always returns Some

    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<Any>",
        },
    };
    println!(
        "{}thread '<unnamed>' panicked at '{}', {}\r",
        termion::screen::ToMainScreen,
        msg,
        location
    );
}
