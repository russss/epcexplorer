use gs1::epc::tid::{decode_tid, decode_xtid_header, XTIDHeader, TID};
use log::warn;
//use std::collections::HashSet;
use std::sync::mpsc;
use std::time;
use log::debug;

pub(crate) enum ReaderType {
    Invelion(invelion::Reader),
    RU5102(ru5102::Reader),
}

pub(crate) struct ScanSettings {
    pub detailed_scan: bool,
}

impl ScanSettings {
    pub fn default() -> ScanSettings {
        ScanSettings {
            detailed_scan: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScanResult {
    pub epc: Vec<u8>,
    pub tid: Option<TID>,
    pub xtid_header: Option<XTIDHeader>,
    pub serial: Option<Vec<u8>>,
    pub rssi: Option<i8>,
    pub antenna: Option<u8>,
    pub last_seen: time::Instant,
}

impl ScanResult {
    pub fn from_epc(epc: Vec<u8>) -> ScanResult {
        ScanResult {
            epc: epc,
            tid: None,
            xtid_header: None,
            serial: None,
            rssi: None,
            antenna: None,
            last_seen: time::Instant::now(),
        }
    }

    pub fn update(&mut self, other: ScanResult) {
        assert_eq!(self.epc, other.epc);
        self.tid = match other.tid {
            Some(tid) => Some(tid),
            None => self.tid,
        };
        self.xtid_header = match other.xtid_header {
            Some(xtid) => Some(xtid),
            None => self.xtid_header,
        };
        self.serial = match other.serial {
            Some(serial) => Some(serial),
            None => self.serial.to_owned(),
        };
        self.rssi = match other.rssi {
            Some(rssi) => Some(rssi),
            None => self.rssi
        };
        self.antenna = other.antenna;
        self.last_seen = other.last_seen;
    }
}

fn read_tid(
    reader: &mut ru5102::Reader,
    uid: &Vec<u8>,
    start: u8,
    words: u8,
) -> Result<Vec<u8>, ru5102::error::Error> {
    let read_cmd = ru5102::ReadCommand {
        epc: uid.to_owned(),
        location: ru5102::MemoryLocation::TID,
        start_address: start,
        count: words,
        password: None,
        mask_address: None,
        mask_length: None,
    };

    reader.read_data(read_cmd)
}

fn get_details_ru5102(
    tags: &[ScanResult],
    reader: &mut ru5102::Reader,
) -> Vec<ScanResult> {
    tags.iter().map(|tag| get_tag_details_ru5102(tag, reader)).collect()
}

fn get_tag_details_ru5102(tag: &ScanResult, reader: &mut ru5102::Reader) -> ScanResult {
    let mut tag = tag.to_owned();
    tag.tid = match read_tid(reader, &tag.epc, 0, 2) {
        Ok(res) => {
            debug!("Read TID: {:?}", res);
            Some(decode_tid(&res).unwrap())
        },
        Err(_err) => None
    };

    tag.xtid_header = match tag.tid {
        Some(tid) => {
            if tid.xtid {
                match read_tid(reader, &tag.epc, 2, 1) {
                    Ok(res) => match decode_xtid_header(&res) {
                        Ok(header) => Some(header),
                        Err(_) => None,
                    },
                    Err(_) => None,
                }
            } else {
                None
            }
        }
        None => None,
    };

    tag.serial = match read_tid(reader, &tag.epc, 2, 3) {
        Ok(data) => Some(data),
        Err(_) => None,
    };

    tag
}

fn scan_ru5102(reader: &mut ru5102::Reader) -> Vec<ScanResult> {
    let mut result = Vec::new();
    let inv = reader.inventory().unwrap();
    if inv.len() > 0 {
        for uid in inv.iter() {
            result.push(ScanResult::from_epc(uid.to_owned()));
        }
    }
    result
}

fn scan_invelion(reader: &mut invelion::Reader) -> invelion::error::Result<Vec<ScanResult>> {
    let mut result = Vec::new();
    for i in 0..4 {
        reader.set_work_antenna(i)?;
        let inv = reader.real_time_inventory(255)?;
        for item in inv.items.iter() {
            let mut res = ScanResult::from_epc(item.epc.to_owned());
            res.rssi = Some(item.rssi);
            res.antenna = Some(item.antenna);
            result.push(res);
        }
    }
    Ok(result)
}

fn get_details_invelion(
    reader: &mut invelion::Reader,
    antenna: u8
) -> invelion::error::Result<Vec<ScanResult>> {
    reader.set_work_antenna(antenna)?;
    let data = reader.read(invelion::protocol::MemoryBank::TID, &[0, 0, 0, 0], 0, 2)?;
    Ok(data.iter().map(|response|
        ScanResult {
            epc: response.epc.to_owned(),
            tid: match decode_tid(&response.data) {
                Ok(tid) => Some(tid),
                Err(_) => {warn!("decode_tid error: {:?}", response.data);
                    None
                }
            },
            rssi: None,
            antenna: Some(response.antenna),
            xtid_header: None,
            serial: None,
            last_seen: time::Instant::now()       
        }
    ).collect())
}

fn scan(reader_type: &mut ReaderType) -> Vec<ScanResult> {
    match reader_type {
        ReaderType::Invelion(reader) => match scan_invelion(reader) {
            Ok(result) => result,
            Err(err) => {
                warn!("Scan error: {:?}", err);
                vec![]
            }
        },
        ReaderType::RU5102(reader) => scan_ru5102(reader),
    }
}

pub(crate) fn scan_thread(
    mut reader_type: ReaderType,
    tx: mpsc::Sender<ScanResult>,
    settings_rx: mpsc::Receiver<ScanSettings>,
) {
    let mut settings = ScanSettings::default();
    let mut detailed_scan_antenna = 0;
    loop {
        match settings_rx.try_recv() {
            Ok(new_settings) => {
                settings = new_settings;
            }
            Err(_) => {}
        }
        let tags = scan(&mut reader_type);
        for tag in tags.iter() {
            tx.send(tag.to_owned()).unwrap();
        }

        if settings.detailed_scan {
            let tags = match &mut reader_type {
                ReaderType::Invelion(reader) => match get_details_invelion(reader, detailed_scan_antenna) {
                    Ok(result) => result,
                    Err(err) => {
                        warn!("Detailed scan error: {:?}", err);
                        vec![]
                    }
                },
                ReaderType::RU5102(reader) => get_details_ru5102(&tags, reader)
            };
            for tag in tags {
                tx.send(tag).unwrap();
            }

            detailed_scan_antenna = (detailed_scan_antenna + 1) % 4;
        }
    }
}
