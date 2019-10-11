mod util;

extern crate clap;
extern crate failure;
extern crate flexi_logger;
extern crate gs1;
extern crate hex;
extern crate invelion;
extern crate log;
extern crate ru5102;
extern crate termion;
extern crate tui;
extern crate backtrace;

mod app;
mod rfid;
mod tagdetail;
mod tagtable;

use std::io;
use std::panic;
use std::sync::mpsc;
use std::thread;
use std::process;

use crate::app::App;
use crate::rfid::{scan_thread, ReaderType, ScanResult, ScanSettings};
use crate::tagdetail::TagDetail;
use crate::tagtable::TagTable;

use clap::{App as Clap, Arg};
use failure::bail;
use log::error;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Layout};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Widget};
use tui::Terminal;
use backtrace::Backtrace;

use crate::util::event::{Event, Events};

fn init_rfid(
    driver: &str,
    port: &str,
) -> Result<(mpsc::Receiver<ScanResult>, mpsc::Sender<ScanSettings>), failure::Error> {
    let reader_type = match driver {
        "ru5102" => ReaderType::RU5102(ru5102::Reader::new(port)?),
        "invelion" => ReaderType::Invelion(invelion::Reader::new(port, 1, 4)?),
        _ => {
            bail!("Invalid reader type (shouldn't happen)");
        }
    };

    let (scan_tx, scan_rx) = mpsc::channel();
    let (settings_tx, settings_rx) = mpsc::channel();
    thread::spawn(move || {
        scan_thread(reader_type, scan_tx, settings_rx);
    });
    Ok((scan_rx, settings_tx))
}

fn main() -> Result<(), failure::Error> {
    let matches = Clap::new("EPC Explorer")
        .arg(
            Arg::with_name("PORT")
                .help("Serial port for reader")
                .required(true),
        )
        .arg(
            Arg::with_name("DRIVER")
                .help("Driver to use")
                .possible_values(&["ru5102", "invelion"])
                .required(true),
        )
        .arg(
            Arg::with_name("log")
                .short("l")
                .value_name("DIRECTORY")
                .help("Write debug logs to files in DIRECTORY")
                .takes_value(true),
        )
        .get_matches();

    if let Some(log_dir) = matches.value_of("log") {
        flexi_logger::Logger::with_str("debug, tui=warn")
            .log_to_file()
            .format(flexi_logger::with_thread)
            .directory(log_dir)
            .start()
            .unwrap();
    }

    let (scan_rx, _settings_tx) = init_rfid(
        matches.value_of("DRIVER").unwrap(),
        matches.value_of("PORT").unwrap(),
    )?;

    panic::set_hook(Box::new(panic_hook));

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    let mut app = App::new();

    loop {
        terminal.draw(|mut f| {
            let items = app.get_items();
            let selected = app.selected.to_owned();
            let selected_item = match selected {
                Some(epc) => app.items.get(&epc),
                None => None,
            };
            let rects = Layout::default()
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(f.size());
            TagTable::new(&items, app.selected.to_owned()).render(&mut f, rects[0]);
            TagDetail::new(selected_item).render(&mut f, rects[1]);
        })?;

        match events.next()? {
            Event::Input(key) => match key {
                Key::Char('q') => {
                    break;
                }
                Key::Char('i') => {
                    app.show_inactive = !app.show_inactive;
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
                app.update_items(&scan_rx);
            }
        };
    }

    Ok(())
}

pub(crate) fn block<'a>(title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Red))
}

fn panic_hook(info: &panic::PanicInfo<'_>) {
    let backtrace = Backtrace::new();
    let thread = thread::current();
    let thread = thread.name().unwrap_or("unnamed");

    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<Any>",
        },
    };
    match info.location() {
        Some(location) => {
            error!(
                target: "panic", "thread '{}' panicked at '{}': {}:{}\n{:?}",
                thread,
                msg,
                location.file(),
                location.line(),
                backtrace
            );
        }
        None => error!(
            target: "panic",
            "thread '{}' panicked at '{}'\n{:?}",
            thread,
            msg,
            backtrace
        ),
    }
    println!(
        "{}thread '<unnamed>' panicked at '{}'. See log for more info.\r",
        termion::screen::ToMainScreen,
        msg
    );
    process::exit(1);
}
