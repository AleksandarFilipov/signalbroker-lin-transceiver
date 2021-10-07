use crate::record::Record;

pub struct Records {
    record_list: Vec<Record>,
}

impl Records {
    pub fn new() -> Self {
        Self {
            record_list: Vec::with_capacity(30),
        }
    }

    pub fn set_list(&mut self, records: Vec<Record>) {
        self.record_list = records;
    }

    pub fn find_by_id_im(&self, id: u8) -> Option<Record> {
        for record in &self.record_list {
            if record.id() == id {
                return Some(record.clone());
            }
        }
        None
    }

    pub fn find_by_id(&mut self, id: u8) -> Option<&mut Record> {
        for record in &mut self.record_list {
            if record.id() == id {
                return Some(record);
            }
        }

        None
    }
}
