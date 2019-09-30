use crate::rfid::ScanResult;
use crate::block;
use tui::layout::Rect;
use tui::buffer::Buffer;
use tui::widgets::{Widget, Table, Row};
use tui::style::{Color, Style, Modifier};
use gs1::{epc, epc::tid::mdid_name, epc::tid::tmid_name};

fn render_row(item: &ScanResult) -> Vec<String> {
    let mut epc_str = match epc::decode_binary(&item.epc) {
        Ok(val) => val.to_uri(),
        Err(_) => hex::encode_upper(&item.epc)
    };

    if epc_str == "urn:epc:id:unprogrammed" {
        epc_str = hex::encode_upper(&item.epc);
    }

    vec![
        epc_str,
        match item.tid {
            Some(tid) => mdid_name(&tid.mdid).to_string(),
            None => "".to_string()
        },
        match item.tid {
            Some(tid) => match tmid_name(&tid.mdid, &tid.tmid) {
                "Unknown" => format!("0x{:X}", &tid.tmid),
                found => found.to_string()
            },
            None => "".to_string()
        },
        match item.xtid_header {
            Some(_) => "Y",
            None => ""
        }.to_string(),
        match item.serial {
            Some(_) => "Y",
            None => ""
        }.to_string(),
        format!("{}s", item.last_seen.elapsed().as_secs())
    ]
}


pub(crate) struct TagTable<'a> {
    pub items: &'a Vec<&'a ScanResult>,
    pub selected: Option<Vec<u8>>
}

impl<'a> TagTable<'a> {
    pub fn new(items: &'a Vec<&'a ScanResult>, selected: Option<Vec<u8>>) -> TagTable<'a> {
        TagTable {
            items: items,
            selected: selected
        }
    }
}

impl<'a> Widget for TagTable<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let header = ["ID", "Manufacturer", "Model", "XTID", "Serial", "Age"];
        let selected_style = Style::default().fg(Color::Yellow);
        let normal_style = Style::default();
        let rows = self.items.iter().map(|item| {
            let mut style = normal_style;
            if item.last_seen.elapsed().as_secs() > 2 {
                style = style.fg(Color::Gray);
            }
            match &self.selected {
                Some(selected) => {
                    if &item.epc == selected {
                        style = selected_style;
                    }
                }
                None => {}
            };

            let cols = render_row(item).into_iter();
            Row::StyledData(cols, style)
        });
        Table::new(header.into_iter(), rows)
            .header_style(Style::default().modifier(Modifier::BOLD))
            .block(block("Tags"))
            .widths(&[50, 25, 10, 6, 6, 9])
            .draw(area, buf);
    }
}
