/// A heap-allocated intermediate buffer for USB bulk data.
///
/// Bulk endpoint data is received in packets of up to 512 Bytes, but we want to offer a
/// stream-based interface to the user, where arbitrarily small amounts of data can be `Read`
/// through. This type provides that interface.
pub struct Buffer {
    inner: Box<[u8]>,
}

impl Buffer {
    pub const SIZE: usize = 4096;

    pub fn new() -> Self {
        Self {
            inner: vec![0; Self::SIZE].into_boxed_slice(),
        }
    }

    pub fn free_space(&self) -> usize {
        0
    }

    pub fn append(&mut self, data: &[u8]) {}
}
