use core::{
    alloc::{self, Layout},
    borrow::Borrow,
    error,
    fmt,
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::{self, NonNull},
};

use crate::mem_alloc::{AllocAddr, TrMalloc};

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
    T: ?Sized,
{
    type AllocErr: error::Error;
    type DeallocErr: error::Error;

    fn allocate<F>(&self, emplace: F) -> Result<NonNull<T>, Self::AllocErr>
    where
        T: Sized,
        F: FnOnce(&mut MaybeUninit<T>);

    fn allocate_slice<F>(
        &self,
        length: usize,
        emplace_items: F,
    ) -> Result<NonNull<[T]>, Self::AllocErr>
    where
        T: Sized,
        F: FnMut(usize, &mut MaybeUninit<T>);

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
    M: TrMalloc + fmt::Debug,
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
    AddressErr(AllocAddr),
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
    M: TrMalloc + fmt::Debug,
{
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        if let Self::MemDeallocErr(e) = self {
            e.source()
        } else {
            Option::None
        }
    }
}

/// A typed allocator by wrapping a `TrMalloc`.
#[derive(Debug)]
pub struct TypedAllocator<T, B, M>
where
    T: ?Sized,
    B: Borrow<M>,
    M: TrMalloc,
{
    _unused_t_: PhantomData<T>,
    _unused_m_: PhantomData<M>,
    mem_alloc_: B,
}

impl<'a, T, M> TypedAllocator<T, &'a M, M>
where
    T: ?Sized,
    M: TrMalloc,
{
    #[inline]
    pub const fn new_by_borrowing(mem_alloc: &'a M) -> Self {
        TypedAllocator::new(mem_alloc)
    }
}

impl<T, M> TypedAllocator<T, M, M>
where
    T: ?Sized,
    M: TrMalloc,
{
    #[inline]
    pub const fn new_by_owning(mem_alloc: M) -> Self {
        TypedAllocator::new(mem_alloc)
    }

    #[inline]
    pub fn into_mem_alloc(self) -> M {
        self.mem_alloc_
    }
}

impl<T, B, M> TypedAllocator<T, B, M>
where
    T: ?Sized,
    B: Borrow<M>,
    M: TrMalloc,
{
    #[inline]
    pub const fn new(mem_alloc: B) -> Self {
        TypedAllocator {
            _unused_t_: PhantomData,
            _unused_m_: PhantomData,
            mem_alloc_: mem_alloc,
        }
    }

    #[inline]
    pub fn mem_allocator(&self) -> &M {
        self.mem_alloc_.borrow()
    }
}

impl<T, B, M> AsRef<M> for TypedAllocator<T, B, M>
where
    T: ?Sized,
    B: Borrow<M>,
    M: TrMalloc,
{
    #[inline]
    fn as_ref(&self) -> &M {
        self.mem_allocator()
    }
}

unsafe impl<T, B, M> TrTypedAlloc<T> for TypedAllocator<T, B, M>
where
    T: ?Sized,
    B: Borrow<M>,
    M: TrMalloc + fmt::Debug,
{
    type AllocErr = TypedAllocError<M>;
    type DeallocErr = TypedDeallocError<M>;

    fn allocate<F>(&self, emplace: F) -> Result<NonNull<T>, Self::AllocErr>
    where
        T: Sized,
        F: FnOnce(&mut MaybeUninit<T>),
    {
        let ptr = self
            .mem_alloc_
            .borrow()
            .allocate(Layout::new::<T>())
            .map_err(|e| TypedAllocError::MemAllocErr(e))?;
        // This line requires unstable feature "slice_ptr_get"
        let data_ptr = ptr.as_non_null_ptr().as_ptr() as *mut T;
        let r = unsafe {
            let p = data_ptr as *mut MaybeUninit<T>;
            emplace(p.as_mut_unchecked());
            NonNull::new_unchecked(data_ptr)
        };
        Result::Ok(r)
    }

    fn allocate_slice<F>(
        &self,
        length: usize,
        mut emplace_items: F,
    ) -> Result<NonNull<[T]>, Self::AllocErr>
    where
        T: Sized,
        F: FnMut(usize, &mut MaybeUninit<T>),
    {
        let layout = Layout::array::<T>(length)
            .map_err(|e| TypedAllocError::LayoutErr(e))?;
        let ptr = self
            .mem_alloc_
            .borrow()
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
                emplace_items(index, item)
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
            let ptr = NonNull::new_unchecked(data as *mut u8);
            self.mem_alloc_
                .borrow()
                .deallocate(ptr, layout)
                .map_err(|e| TypedDeallocError::MemDeallocErr(e))
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
            let ptr = NonNull::new_unchecked(slice.as_ptr() as *mut u8);
            self.mem_alloc_
                .borrow()
                .deallocate(ptr, layout)
                .map_err(|e| TypedDeallocError::MemDeallocErr(e))
        }
    }
}

#[cfg(test)]
mod tests_ {
    use core::mem::MaybeUninit;

    use crate::{
        core_alloc_::CoreAlloc,
        typed_alloc::{TrTypedAlloc, TypedAllocator},
    };

    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    struct TestData {
        a: usize,
        b: usize,
    }

    #[test]
    fn typed_alloc_can_alloc_dealloc_fixed_size() {
        let d = TestData { a: 0x12345678, b: 0x9abcdef0 };
        let a = TypedAllocator::<TestData, _, _>::new_by_owning(CoreAlloc::new());
        let r = a.allocate(|p| {
            p.write(d);
        });
        let ptr: core::ptr::NonNull<TestData> = r.unwrap();
        let pdata = unsafe { ptr.as_ref() };
        assert_eq!(pdata, &d);

        let r = unsafe { a.deallocate(ptr) };
        assert!(r.is_ok())
    }

    #[test]
    fn typed_alloc_can_alloc_dealloc_dyn_size() {
        let a = TypedAllocator::<TestData, _, _>::new_by_owning(CoreAlloc::new());
        let r = a.allocate_slice(16, |i: usize, p: &mut MaybeUninit<TestData>| {
            p.write(TestData { a: usize::MAX - i, b: 1 << i });
        });
        let ptr = r.unwrap();
        let slice = unsafe { ptr.as_ref() };

        for (i, data) in slice.iter().enumerate() {
            let sample = TestData { a: usize::MAX - i, b: 1 << i };
            assert_eq!(data, &sample);
        }

        let r = unsafe { a.deallocate_slice(ptr) };
        assert!(r.is_ok())
    }
}
