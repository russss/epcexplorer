use gs1::epc::tid::{decode_tid, decode_xtid_header, XTIDHeader, TID};
use std::sync::mpsc;
use std::thread::sleep;
use std::time;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScanResult {
    pub epc: Vec<u8>,
    pub tid: Option<TID>,
    pub xtid_header: Option<XTIDHeader>,
    pub serial: Option<Vec<u8>>,
    pub last_seen: time::Instant,
}

impl ScanResult {
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

fn read_tag(reader: &mut ru5102::Reader, uid: &Vec<u8>) -> ScanResult {
    let tid = match read_tid(reader, &uid, 0, 2) {
        Ok(res) => Some(decode_tid(&res).unwrap()),
        Err(_err) => None,
    };

    let xtid = match tid {
        Some(tid) => {
            if tid.xtid {
                match read_tid(reader, &uid, 2, 1) {
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

    let serial = match read_tid(reader, &uid, 2, 3) {
                    Ok(data) => Some(data),
                    Err(_) => None,
    };

    ScanResult {
        epc: uid.to_vec(),
        tid: tid,
        xtid_header: xtid,
        serial: serial,
        last_seen: time::Instant::now(),
    }
}

pub(crate) fn scan_thread(mut reader: ru5102::Reader, tx: mpsc::Sender<ScanResult>) {
    loop {
        let inv = reader.inventory().unwrap();
        if inv.len() > 0 {
            for uid in inv.iter() {
                tx.send(read_tag(&mut reader, uid)).unwrap();
            }
        }
        sleep(time::Duration::from_millis(50));
    }
}
