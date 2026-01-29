use core::{
    alloc::{self, Layout},
    error,
    fmt,
    mem::MaybeUninit,
    ptr::{self, NonNull},
};

use crate::mem_alloc::TrMalloc;

/// Functions that initialize dynamic sized data at a given address, as if it
/// was constructed directly at place.
/// 
/// # Safety
/// 
/// see [TrEmplace::at] for detail.
pub unsafe trait TrEmplace<T>
where
    T: ?Sized,
{
    /// Consumes self to initialize data at the address given by pointer to 
    /// the uninit memory.
    /// 
    /// # Safety
    /// 
    /// * Memory pointed by the `uninit` argument should assumed uninitialized.
    ///   Reading any data prior to the initialization will cause UB.
    unsafe fn at(self, uninit: NonNull<T>);
}

pub trait TrEmplaceItems<T> {
    fn at_index(
        &mut self,
        index: usize,
        uninit: &mut MaybeUninit<T>,
    );
}

unsafe impl<F, T> TrEmplace<T> for F
where
    F: FnOnce(NonNull<T>),
    T: ?Sized,
{
    unsafe fn at(self, uninit: NonNull<T>) {
        self(uninit)
    }
}

impl<F, T> TrEmplaceItems<T> for F
where
    F: FnMut(usize, &mut MaybeUninit<T>),
{
    fn at_index(
        &mut self,
        index: usize,
        uninit: &mut MaybeUninit<T>,
    ) {
        self(index, uninit)
    }
}

/// Memory allocator for a concrete SIZED type.
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
pub unsafe trait TrTypedAlloc<T>
where
    Self: fmt::Debug,
    T: ?Sized,
{
    type AllocErr: error::Error;
    type DeallocErr: error::Error;

    fn allocate<F>(&self, emplace: F) -> Result<NonNull<T>, Self::AllocErr>
    where
        T: Sized,
        F: TrEmplace<T>;

    fn allocate_slice<F>(
        &self,
        length: usize,
        emplace_items: &mut F,
    ) -> Result<NonNull<[T]>, Self::AllocErr>
    where
        T: Sized,
        F: TrEmplaceItems<T>;

    /// Deallocates the storage referenced by the `pointer`, which must be a 
    /// pointer obtained by an earlier call to allocate().
    /// 
    /// Item of type `T` will drop before its storage is being reclaimed.
    /// 
    /// # Safety
    /// 
    /// * The argument pointer must be equal to the first call to allocate() 
    ///   that originally produced.
    /// * The data pointed by the argument pointer must be at a legal state and
    ///   no any other references.
    unsafe fn deallocate(
        &self,
        pointer: NonNull<T>,
    ) -> Result<usize, Self::DeallocErr>;

    /// Deallocates the storage of the slice referenced by the `slice_pointer`, 
    /// which must be a pointer obtained by an earlier call to `allocate_slice`.
    /// 
    /// Each items of type `T` will drop in the order of its construction
    /// as in `allocate_slice`, before their storage is being reclaimed.
    /// 
    /// # Safety
    /// 
    /// * The argument pointer must be equal to the first call to
    ///   `allocate_slice` that originally produced.
    unsafe fn deallocate_slice(
        &self,
        slice_pointer: NonNull<[T]>, 
    ) -> Result<usize, Self::DeallocErr>
    where
        T: Sized;
}

#[derive(Debug)]
pub enum TypedAllocError<M>
where
    M: TrMalloc,
{
    LayoutErr(alloc::LayoutError),
    MemAllocErr(<M as TrMalloc>::AllocErr),
}

impl<M> Clone for TypedAllocError<M>
where
    M: TrMalloc,
    <M as TrMalloc>::AllocErr: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::LayoutErr(e) => TypedAllocError::LayoutErr(e.clone()),
            Self::MemAllocErr(e) => TypedAllocError::MemAllocErr(e.clone()),
        }
    }
}

impl<M> fmt::Display for TypedAllocError<M>
where
    M: TrMalloc,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayoutErr(e)
                => write!(f, "AllocError::LayoutErr: {e}"),
            Self::MemAllocErr(e)
                => write!(f, "AllocError::MemAllocErr: {e}"),
        }
    }
}

impl<M> error::Error for TypedAllocError<M>
where
    M: TrMalloc,
{
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::LayoutErr(e) => e.source(),
            Self::MemAllocErr(e) => e.source(),
        }
    }
}

#[derive(Debug)]
pub enum TypedDeallocError<M>
where
    M: TrMalloc,
{
    AddressErr(NonNull<()>),
    MemDeallocErr(<M as TrMalloc>::DeallocErr),
}

impl<M> Clone for TypedDeallocError<M>
where
    M: TrMalloc,
    <M as TrMalloc>::DeallocErr: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::AddressErr(e)
                => TypedDeallocError::AddressErr(*e),
            Self::MemDeallocErr(e)
                => TypedDeallocError::MemDeallocErr(e.clone()),
        }
    }
}

impl<M> fmt::Display for TypedDeallocError<M>
where
    M: TrMalloc,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressErr(e)
                => write!(f, "TypedDeallocError::AddressErr: {e:?}"),
            Self::MemDeallocErr(e)
                => write!(f, "TypedDeallocError::MemDeallocErr: {e}"),
        }
    }
}

impl<M> error::Error for TypedDeallocError<M>
where
    M: TrMalloc,
{
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        if let Self::MemDeallocErr(e) = self {
            e.source()
        } else {
            Option::None
        }
    }
}

unsafe impl<M, T> TrTypedAlloc<T> for M
where
    T: ?Sized,
    M: TrMalloc + Clone,
{
    type AllocErr = TypedAllocError<M>;
    type DeallocErr = TypedDeallocError<M>;

    fn allocate<F>(&self, emplace: F) -> Result<NonNull<T>, Self::AllocErr>
    where
        T: Sized,
        F: TrEmplace<T>,
    {
        let ptr = self
            .allocate(Layout::new::<T>())
            .map_err(|e| TypedAllocError::MemAllocErr(e))?;
        // This line requires unstable feature "slice_ptr_get"
        let data_ptr = ptr.as_non_null_ptr().as_ptr() as *mut T;
        let r = unsafe {
            let p = NonNull::new_unchecked(data_ptr);
            emplace.at(p);
            NonNull::new_unchecked(data_ptr)
        };
        Result::Ok(r)
    }

    fn allocate_slice<F>(
        &self,
        length: usize,
        emplace_items: &mut F,
    ) -> Result<NonNull<[T]>, Self::AllocErr>
    where
        T: Sized,
        F: TrEmplaceItems<T>,
    {
        let layout = Layout::array::<T>(length)
            .map_err(|e| TypedAllocError::LayoutErr(e))?;
        let ptr = self
            .allocate(layout)
            .map_err(|e| TypedAllocError::MemAllocErr(e))?;
        let slice_ptr = {
            let data_ptr = ptr.as_non_null_ptr().as_ptr() as *mut T;
            ptr::slice_from_raw_parts_mut(data_ptr, length)
        };
        let r = unsafe {
            let uninit_slice_ptr = slice_ptr as *mut [MaybeUninit<T>];
            let uninit_slice = &mut *uninit_slice_ptr;
            for (index, item) in uninit_slice.iter_mut().enumerate() {
                emplace_items.at_index(index, item)
            }
            NonNull::new_unchecked(slice_ptr)
        };
        Result::Ok(r)
    }

    unsafe fn deallocate(
        &self,
        pointer: NonNull<T>,
    ) -> Result<usize, Self::DeallocErr> {
        let data = pointer.as_ptr();
        unsafe {
            let layout = Layout::for_value_raw(data);
            data.drop_in_place();
            let ptr = ptr::slice_from_raw_parts_mut(
                data as *mut u8,
                layout.size(),
            );
            let ptr = NonNull::new_unchecked(ptr);
            let addr = NonNull::new_unchecked(data as *mut u8 as *mut ());
            self.deallocate(ptr, layout)
                .map_err(|_| TypedDeallocError::AddressErr(addr))
        }
    }

    unsafe fn deallocate_slice(
        &self,
        slice_pointer: NonNull<[T]>,
    ) -> Result<usize, Self::DeallocErr>
    where
        T: Sized,
    {
        let slice = unsafe { slice_pointer.as_ref() };
        for item in slice.iter() {
            let p_item = item as *const _ as *mut T;
            unsafe { p_item.drop_in_place() };
        };
        unsafe {
            let layout = Layout::for_value(slice);
            let addr = slice.as_ptr() as *mut u8;
            let ptr = ptr::slice_from_raw_parts_mut(addr, layout.size());
            let ptr = NonNull::new_unchecked(ptr);
            self.deallocate(ptr, layout)
                .map_err(|e| TypedDeallocError::MemDeallocErr(e))
        }
    }
}

#[cfg(test)]
mod tests_ {
    use core::{
        alloc::Layout,
        ptr::NonNull,
    };

    use crate::mem_alloc::TrMalloc;

    #[derive(Debug, Clone, Copy)]
    enum TestMemAllocErr {
        UnsupportedLayout(Layout),
        IncapableAlloc(),
        InvalidPointer(NonNull<[u8]>),
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct TestMemAlloc {}
}