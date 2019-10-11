use crate::rfid::ScanResult;
use std::cmp;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time;

const INACTIVE_AGE: time::Duration = time::Duration::from_secs(5);

pub(crate) struct App {
    pub items: HashMap<Vec<u8>, ScanResult>,
    pub selected: Option<Vec<u8>>,
    pub show_inactive: bool,
}

impl ScanResult {
    fn active(&self) -> bool {
        self.last_seen.elapsed() < INACTIVE_AGE
    }
}

impl App {
    pub fn new() -> App {
        App {
            items: HashMap::new(),
            selected: None,
            show_inactive: false,
        }
    }

    pub fn update_items(&mut self, rx: &mpsc::Receiver<ScanResult>) {
        loop {
            match rx.try_recv() {
                Ok(result) => {
                    let epc = result.epc.to_vec();
                    match self.items.get_mut(&epc) {
                        Some(item) => {
                            item.update(result);
                        }
                        None => {
                            self.items.insert(epc, result);
                        }
                    };
                }
                Err(_) => {
                    break;
                }
            };
        }
        let items = self.get_items();
        if items.len() > 0 {
            match self.selected {
                None => {
                    self.selected = Some(items[0].epc.to_vec());
                }
                _ => {}
            }
        } else {
            self.selected = None;
        }
    }

    pub fn update_selected(&mut self, reverse: bool) {
        let items = self.get_items();

        let selected = match &self.selected {
            Some(epc) => epc.to_vec(),
            None => {
                self.selected = Some(items[0].epc.to_vec());
                return;
            }
        };

        let mut selected_index = 0;
        for (i, item) in items.iter().enumerate() {
            if item.epc == selected {
                selected_index = i;
                break;
            }
        }

        if selected_index == 0 && reverse {
            selected_index = items.len() - 1;
        } else if selected_index == items.len() - 1 && !reverse {
            selected_index = 0;
        } else if reverse {
            selected_index -= 1;
        } else {
            selected_index += 1;
        }

        self.selected = Some(items[selected_index].epc.to_vec());
    }

    pub fn get_items(&self) -> Vec<&ScanResult> {
        let mut items: Vec<&ScanResult> = self.items.iter().filter_map(
            |(_, item)| 
            if !item.active() && !self.show_inactive {
                None
            } else {
                Some(item)
            }
        ).collect();
        items.sort_by_key(|res| {
            (
                cmp::max(res.last_seen.elapsed(), INACTIVE_AGE),
                res.epc.to_owned(),
            )
        });
        items
    }
}
