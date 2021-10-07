use std::io::Write;

/// A record held LIN-frame data.
///
/// `id` is equal to a LIN-frame ID
///
/// `size` is equal to a LIN-frame size
///
/// `master` tells if the frame is a master frame(1) or an arbitration frame(0)
///
/// `cache_valid` tells if the cached_data is valid data for this record
#[derive(Clone, Debug)]
pub struct Record {
    id: u8,
    size: u8,
    master: u8,
    cache_valid: bool,
    write_cache: Vec<u8>,
}

impl Record {
    pub fn new(id: u8, size: u8, master: u8) -> Self {
        Self {
            id,
            size,
            master,
            cache_valid: false,
            write_cache: Vec::with_capacity(8),
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
        self.write_cache.clear();
        self.write_cache.write_all(data).unwrap();
    }

    pub fn cache(&self) -> &[u8] {
        &self.write_cache[..self.size as usize]
    }
}
