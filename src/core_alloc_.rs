extern crate alloc;

use alloc::alloc::{alloc, dealloc};

use core::{
    alloc::{Allocator, AllocatorClone, AllocError, Layout},
    ptr::{self, NonNull},
};


/// A wrapper for `alloc::alloc` and `alloc::dealloc`
#[derive(Debug, Default, Clone, Copy)]
pub struct CoreAlloc;

impl CoreAlloc {
    pub const fn new() -> Self {
        CoreAlloc
    }

    pub fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            let Option::Some(p) = NonNull::new(alloc(layout)) else {
                return Result::Err(AllocError);
            };
            #[cfg(test)]
            log::trace!(
                "[CoreAlloc::allocate]({}, {}) returns {:?}",
                layout.size(),
                layout.align(),
                p.as_ptr()
            );
            let slice = ptr::slice_from_raw_parts_mut(
                p.as_ptr(),
                layout.size(),
            );
            Result::Ok(NonNull::new_unchecked(slice))
        }
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// * `ptr` must denote a block of memory [*currently allocated*] via this
    ///   allocator, and
    /// * `layout` must [*fit*] that block of memory.
    ///
    /// [*currently allocated*]: #currently-allocated-memory
    /// [*fit*]: #memory-fitting
    pub unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        #[cfg(test)]
        log::trace!(
            "[CoreAlloc::deallocate]({:?}), layout: ({}, {})",
            ptr.as_ptr(),
            layout.size(),
            layout.align()
        );
        unsafe { dealloc(ptr.as_ptr(), layout) };
    }
}

unsafe impl Allocator for CoreAlloc {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        CoreAlloc::allocate(self, layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { CoreAlloc::deallocate(self, ptr, layout) };
    }
}

unsafe impl AllocatorClone for CoreAlloc {}
