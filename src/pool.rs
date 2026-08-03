#[derive(Debug, Clone)]
pub struct Pool<T> {
    free_list: Vec<T>,
}

impl<T: Default> Pool<T> {
    /// Create a new pool and pre-allocate it with `pre_allocate` items.
    pub fn new(pre_allocate: usize) -> Self {
        let mut pool = Self {
            free_list: Vec::with_capacity(pre_allocate),
        };
        for _ in 0..pre_allocate {
            pool.free_list.push(T::default());
        }
        pool
    }

    /// Acquire an item from the pool. Creates a new item if the pool is empty.
    pub fn acquire(&mut self) -> T {
        self.free_list.pop().unwrap_or_default()
    }

    /// Release an item back to the pool to be recycled.
    pub fn release(&mut self, item: T) {
        self.free_list.push(item);
    }

    /// Returns the number of available items in the pool's free list.
    pub fn len(&self) -> usize {
        self.free_list.len()
    }

    /// Returns true if there are no items currently available in the free list.
    pub fn is_empty(&self) -> bool {
        self.free_list.is_empty()
    }

    /// Returns the number of available items in the pool's free list.
    pub fn available(&self) -> usize {
        self.free_list.len()
    }
}
