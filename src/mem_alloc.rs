use core::{
    alloc::{Layout, LayoutError},
    error,
    fmt,
    ptr::NonNull,
};

pub(crate) type AllocAddr = NonNull<[u8]>;

/// A trait for general purpose allocator to acquire memory fitting the given
/// layout.
///
/// # Safety
///
/// * Memory blocks returned from an allocator must point to valid memory and
///   retain their validity until the instance and all of its clones are
///   dropped,
///
/// * cloning or moving the allocator must not invalidate memory blocks returned
///   from this allocator. A cloned allocator must behave like the same
///   allocator, and
///
/// * any pointer to a memory block which is currently allocated may be passed
///   to any other method of the allocator.
pub unsafe trait TrMalloc {
    type AllocErr: error::Error;
    type DeallocErr: error::Error;

    fn allocate(
        &self,
        layout: Layout,
    ) -> Result<AllocAddr, Self::AllocErr>;

    /// Deallocate memory pointed by the pointer
    ///
    /// # Safety
    ///
    /// The `ptr` must point to a valid address allocated by this allocator.
    unsafe fn deallocate(
        &self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<usize, Self::DeallocErr>;
}

/// A dummy allocator that will do nothing but only return error.
#[derive(Debug, Clone, Default)]
pub struct FakeMalloc;

impl FakeMalloc {
    pub fn shared() -> &'static FakeMalloc {
        static FAKE_ALLOC: FakeMalloc = FakeMalloc;
        &FAKE_ALLOC
    }

    /// Will always returns false for any layout
    pub fn can_support(&self, _: Layout) -> bool {
        false
    }

    /// It will always return `Err`
    pub fn allocate(&self, _: Layout) -> Result<AllocAddr, FakeMallocError> {
        Result::Err(FakeMallocError)
    }

    /// Do nothing but return `Result::Ok(0)`
    ///
    /// # Safety
    ///
    /// - This is not designed to be called manually.
    pub unsafe fn deallocate(
        &self,
        _: NonNull<u8>,
        _: Layout,
    ) -> Result<usize, FakeMallocError> {
        Result::Ok(0usize)
    }
}

/// No matter `allocate` or `deallocate`, you are doing it all wrong!
#[derive(Debug, Clone, Copy, Default)]
pub struct FakeMallocError;

unsafe impl TrMalloc for FakeMalloc {
    type AllocErr = FakeMallocError;
    type DeallocErr = FakeMallocError;

    #[inline]
    fn allocate(&self, layout: Layout) -> Result<AllocAddr, FakeMallocError> {
        FakeMalloc::allocate(self, layout)
    }

    #[inline]
    unsafe fn deallocate(
        &self,
        ptr: NonNull<u8>,
        layout: Layout,
    ) -> Result<usize, FakeMallocError> {
        unsafe { FakeMalloc::deallocate(self, ptr, layout) }
    }
}

impl fmt::Display for FakeMallocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FakeMallocError")
    }
}

impl error::Error for FakeMallocError {}

#[derive(Debug, Default, Clone)]
pub enum AllocError {
    UnsupportedLayout(Layout),
    LayoutErr(LayoutError),
    NullPtrReturned,
    #[default]Unknown,
}

#[derive(Debug, Default, Clone)]
pub enum DeallocError {
    InvalidAddr(AllocAddr),
    #[default]Unknown,
}


impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedLayout(layout)
                => write!(f, "AllocError::UnsupportedLayout({layout:?})"),
            Self::LayoutErr(err)
                => write!(f, "AllocError::LayoutErr({err:?})"),
            Self::NullPtrReturned
                => write!(f, "AllocError::NullPtrReturned"),
            Self::Unknown
                => write!(f, "AllocError::Unknown"),
        }
    }
}

impl error::Error for AllocError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        if let AllocError::LayoutErr(err) = self {
            Option::Some(err)
        } else {
            Option::None
        }
    }
}

impl fmt::Display for DeallocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddr(addr)
                => write!(f, "DeallocError::InvalidAddr({addr:?})"),
            Self::Unknown
                => write!(f, "DeallocError::Unknown"),
        }
    }
}

impl error::Error for DeallocError {}

#[cfg(any(test, feature = "core_alloc"))]
pub use crate::core_alloc_::{CoreAlloc, MemAllocator};
