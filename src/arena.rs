#[derive(Debug, Clone)]
pub struct FrameArena {
    buffer: Vec<u8>,
    offset: usize,
    capacity: usize,
}

impl FrameArena {
    /// Create a new frame arena with a specific byte capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            offset: 0,
            capacity,
        }
    }

    /// Bump-allocate a byte slice from the arena. Returns None if full.
    pub fn alloc(&mut self, size: usize) -> Option<&mut [u8]> {
        if self.offset + size > self.capacity {
            return None;
        }
        let start = self.offset;
        self.offset += size;
        Some(&mut self.buffer[start..self.offset])
    }

    /// Bump-allocate a typed object. Respects memory alignment.
    pub fn alloc_typed<T: Copy + Default>(&mut self) -> Option<&mut T> {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();
        let ptr = self.buffer.as_mut_ptr();
        let current_addr = unsafe { ptr.add(self.offset) } as usize;
        let aligned_addr = (current_addr + align - 1) & !(align - 1);
        let align_offset = aligned_addr - ptr as usize;

        if align_offset + size > self.capacity {
            return None;
        }
        self.offset = align_offset + size;

        unsafe {
            let obj_ptr = ptr.add(align_offset) as *mut T;
            obj_ptr.write(T::default());
            Some(&mut *obj_ptr)
        }
    }

    /// Reset the arena offset, effectively freeing all bump-allocated data instantly.
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    /// Number of bytes currently bump-allocated.
    pub fn used(&self) -> usize {
        self.offset
    }

    /// Remaining capacity in the arena.
    pub fn remaining(&self) -> usize {
        self.capacity - self.offset
    }
}
