use crate::rfid::ScanResult;
use crate::block;
use tui::layout::Rect;
use gs1::epc;
use tui::buffer::Buffer;
use tui::style::{Style, Modifier};
use tui::widgets::{Widget, Text, Paragraph};

pub(crate) struct TagDetail<'a> {
    pub item: Option<&'a ScanResult>
}

impl<'a> TagDetail<'a> {
    pub fn new(item: Option<&'a ScanResult>) -> TagDetail<'a> {
        TagDetail {
            item: item
        }
    }
}

fn render_detail(item: &ScanResult) -> Vec<Text> {
    let mut header = format!("Tag ID: {}", hex::encode_upper(&item.epc));
    match epc::decode_binary(&item.epc) {
        Ok(val) => {
            header.push_str(&format!(" ({})", val.to_uri()));
        }
        Err(_) => {}
    };
    header.push('\n');

    vec![
        Text::styled(header, Style::default().modifier(Modifier::BOLD)),
        Text::raw(match item.tid {
            Some(tid) => format!("{:?}\n", tid),
            None => "\n".to_string()
        }),
        Text::raw(match item.xtid_header {
            Some(xtid) => format!("{:?}\n", xtid),
            None => "\n".to_string()
        }),
    ]
}

impl<'a> Widget for TagDetail<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {

        let text = match self.item {
            Some(item) => render_detail(&item),
            None => vec![]
        };

        Paragraph::new(text.iter())
            .block(block("Detail"))
            .wrap(true)
            .draw(area, buf);
    }
}
