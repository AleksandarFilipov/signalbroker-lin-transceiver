/// A record held LIN-frame data.
///
/// `id` is equal to a LIN-frame ID
///
/// `size` is equal to a LIN-frame size
///
/// `master` tells if the frame is a master frame(1) or an arbitration frame(0)
///
/// `cache_valid` tells if the cached_data is valid data for this record
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    id: u8,
    size: u8,
    master: u8,
    cache_valid: bool,
    write_cache: Vec<u8>,
}

impl Record {
    /// Create a new object of type [`Record`]
    pub fn new(id: u8, size: u8, master: u8) -> Self {
        Self {
            id,
            size,
            master,
            cache_valid: false,
            write_cache: vec![0x00u8; 8],
        }
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn size(&self) -> u8 {
        self.size
    }

    pub fn is_master(&self) -> bool {
        self.master == 1
    }

    pub fn set_cache_valid(&mut self, cache_valid: bool) {
        self.cache_valid = cache_valid;
    }

    pub fn cache_valid(&self) -> bool {
        self.cache_valid
    }

    pub fn set_write_cache(&mut self, data: &[u8]) {
        // Check that the frame isn't greater then 8
        assert!(data.len() <= 8, "Data frame is greater then 8");
        self.write_cache[..data.len()].copy_from_slice(data);
    }

    pub fn cache(&self) -> &[u8] {
        &self.write_cache[..self.size as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::Record;

    #[test]
    fn get_record_id() {
        let record = Record::new(0x43, 5, 1);
        assert_eq!(0x43, record.id());
    }

    #[test]
    fn get_record_size() {
        let record = Record::new(0x00, 4, 0);
        assert_eq!(4, record.size());
    }

    #[test]
    fn is_master_record() {
        let master_record = Record::new(0x32, 6, 1);
        let slave_record = Record::new(0x12, 8, 0);

        assert!(master_record.is_master());
        assert!(!slave_record.is_master());
    }


    #[test]
    fn is_cache_valid() {
        let mut record = Record::new(0x11, 5, 0);
        assert!(!record.cache_valid());
        record.set_cache_valid(true);
        assert!(record.cache_valid());
    }

    #[test]
    fn test_set_write_cache() {
        let mut record = Record::new(0x00, 4, 0);

        let data = [0x00u8, 0x32, 0x13, 0x23];

        record.set_write_cache(&data);

        for (index, value) in record.cache().iter().enumerate() {
            assert_eq!(data[index], *value);
        }
    }

    #[test]
    #[should_panic]
    fn test_set_write_cache_with_to_big_frame() {
        let mut record = Record::new(0x00, 4, 0);

        let data = [0x00u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        // This should panic, because the maximum frame length on LIN is 8
        record.set_write_cache(&data);
    }
}
