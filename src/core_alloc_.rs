extern crate alloc;

use alloc::alloc::{alloc, dealloc};

use core::{
    alloc::Layout,
    ptr::{self, NonNull},
};

use crate::mem_alloc::{AllocError, DeallocError, TrMalloc};

type MemAddr = NonNull<[u8]>;

/// A wrapper for `alloc::alloc` and `alloc::dealloc`
#[derive(Debug, Default, Clone, Copy)]
pub struct CoreAlloc;

impl CoreAlloc {
    pub fn shared() -> &'static Self {
        static CORE_ALLOC: CoreAlloc = CoreAlloc;
        &CORE_ALLOC
    }

    pub const fn new() -> Self {
        CoreAlloc
    }

    pub fn can_support(&self, layout: Layout) -> bool {
        let _ = layout;
        true
    }

    pub fn allocate(&self, layout: Layout) -> Result<MemAddr, AllocError> {
        unsafe {
            let Option::Some(p) = NonNull::new(alloc(layout)) else {
                return Result::Err(AllocError::NullPtrReturned)
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
    pub unsafe fn deallocate(
        &self,
        ptr: MemAddr,
        layout: Layout,
    ) -> Result<usize, DeallocError> {
        #[cfg(test)]
        log::trace!(
            "[CoreAlloc::deallocate]({:?}) len: {}, layout: ({}, {})",
            ptr.as_ptr(),
            unsafe { ptr.as_ref().len() },
            layout.size(),
            layout.align()
        );
        unsafe { dealloc(ptr.as_ptr() as *mut _, layout); }
        Result::Ok(layout.size())
    }
}

unsafe impl TrMalloc for CoreAlloc {
    type AllocErr = AllocError;
    type DeallocErr = DeallocError;

    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<MemAddr, Self::AllocErr> {
        CoreAlloc::allocate(self, layout)
    }

    #[inline(always)]
    unsafe fn deallocate(
        &self,
        ptr: MemAddr,
        layout: Layout,
    ) -> Result<usize, Self::DeallocErr> {
        unsafe { CoreAlloc::deallocate(self, ptr, layout) }
    }
}
