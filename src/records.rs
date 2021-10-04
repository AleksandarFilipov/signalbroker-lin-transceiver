use crate::record::Record;

pub struct Records {
    record_list: Vec<Record>,
}

impl Records {
    pub fn new() -> Self {
        Self {
            record_list: Vec::with_capacity(30)
        }
    }

    pub fn add_records(&mut self, record: Record) {
        self.record_list.push(record);
    }

    pub fn set_list(&mut self, records: Vec<Record>) {
        self.record_list = records;
    }

    pub fn find_by_id(&self, id: u8) -> Option<Record> {
        for record in &self.record_list {
            if record.id() == id {
                return Some(record.clone());
            }
        }

        None
    }
}