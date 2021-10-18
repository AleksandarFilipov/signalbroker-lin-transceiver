use crate::record::Record;

/// Helds a collection of [`Record`]
pub struct Records {
    record_list: Vec<Record>,
}

impl Records {
    /// Create a new object of [`Records`] which held a list with capacity of 30
    pub fn new() -> Self {
        Self {
            record_list: Vec::with_capacity(30),
        }
    }

    /// Set record_lsit to records
    pub fn set_list(&mut self, records: Vec<Record>) {
        self.record_list = records;
    }

    /// Returning a clone of record_list
    fn get_records(&self) -> Vec<Record> {
        self.record_list.clone()
    }

    /// Search over the collection for a record with `id`, if match return an imutable [`Record`]
    /// otherwise return None
    pub fn find_by_id_im(&self, id: u8) -> Option<Record> {
        for record in &self.record_list {
            if record.id() == id {
                return Some(record.clone());
            }
        }
        None
    }

    /// Search over the collection for for a record with `id`, if match return a mutable reference of [`Record`]
    /// otherwise return None
    pub fn find_by_id(&mut self, id: u8) -> Option<&mut Record> {
        for record in &mut self.record_list {
            if record.id() == id {
                return Some(record);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{record::Record, records::Records};

    #[test]
    fn test_set_list() {
        // Create a Record object
        let mut records = Records::new();

        // Create a list with records
        let list = vec![Record::new(0x00, 5, 0), Record::new(0x02, 2, 1)];

        // Add it to the records object
        records.set_list(list.clone());

        // Assert that everything is in place
        assert_eq!(list[0], records.get_records()[0]);
        assert_eq!(list[1], records.get_records()[1]);
    }

    #[test]
    fn test_find_by_id() {
        // Creata a Record object
        let mut records = Records::new();

        // Create a list with records
        let list = vec![Record::new(0x00, 5, 0), Record::new(0x02, 2, 1)];

        records.set_list(list.clone());

        assert_eq!(records.find_by_id(0x00), Some(&mut list[0].clone()));
        assert_eq!(records.find_by_id(0x01), None);
        assert_eq!(records.find_by_id(0x02), Some(&mut list[1].clone()));
    }

    #[test]
    fn test_find_by_id_im() {
        // Create a Record object
        let mut records = Records::new();

        // Create a list with records
        let list = vec![Record::new(0x00, 5, 0), Record::new(0x02, 2, 1)];

        // Add it to the records object
        records.set_list(list.clone());

        assert_eq!(records.find_by_id_im(0x00), Some(list[0].clone()));
        assert_eq!(records.find_by_id_im(0x01), None);
        assert_eq!(records.find_by_id_im(0x02), Some(list[1].clone()));
    }
}
