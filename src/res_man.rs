extern crate alloc;

use core::ops::{Deref, DerefMut, Try};

use crate::mem_alloc::{CoreAlloc, TrMalloc};

/// Smart pointers that can retrieve its memory allocator.
pub trait TrBoxed
where
    Self: Deref<Target = Self::Item>,
{
    type Item: ?Sized;
    type Malloc: TrMalloc;

    fn try_get_mem_alloc(&self) -> impl Try<Output = &Self::Malloc>;
}

/// A trait describing smart pointers that share the ownership of the resource
/// with reference counting.
pub trait TrStrongShared
where
    Self: TrBoxed + Clone,
{
    type Downgraded: TrWeakShared<Item = Self::Item>;

    fn strong_count(&self) -> usize;

    fn try_get_weak_count(&self) -> impl Try<Output = usize>;

    fn downgrade(&self) -> Self::Downgraded;
}

pub trait TrWeakShared
where
    Self: Clone,
{
    type Item: ?Sized;
    type Upgraded: TrStrongShared<Item = Self::Item>;

    fn try_get_strong_count(&self) -> impl Try<Output = usize>;

    fn weak_count(&self) -> usize;

    fn try_upgrade(&self) -> impl Try<Output = Self::Upgraded>;
}

/// A trait describing smart pointers with a unique owner.
pub trait TrUnique
where
    Self: TrBoxed + DerefMut<Target = Self::Item>,
{}

#[cfg(feature = "arc")]
impl<T, A> TrBoxed for alloc::sync::Arc<T, A>
where
    T: ?Sized,
    A: core::alloc::Allocator,
{
    type Item = T;
    type Malloc = CoreAlloc;

    #[inline]
    fn try_get_mem_alloc(&self) -> impl Try<Output = &Self::Malloc> {
        Option::Some(CoreAlloc::shared())
    }
}

#[cfg(feature = "arc")]
impl<T, A> TrStrongShared for alloc::sync::Arc<T, A>
where
    T: ?Sized,
    A: core::alloc::AllocatorClone,
{
    type Downgraded = alloc::sync::Weak<T, A>;

    #[inline]
    fn strong_count(&self) -> usize {
        alloc::sync::Arc::strong_count(self)
    }

    #[inline]
    fn try_get_weak_count(&self) -> impl Try<Output = usize> {
        Option::Some(alloc::sync::Arc::weak_count(self))
    }

    #[inline]
    fn downgrade(&self) -> Self::Downgraded {
        alloc::sync::Arc::downgrade(self)
    }
}

#[cfg(feature = "arc")]
impl<T, A> TrWeakShared for alloc::sync::Weak<T, A>
where
    T: ?Sized,
    A: core::alloc::AllocatorClone,
{
    type Upgraded = alloc::sync::Arc<T, A>;
    type Item = T;

    #[inline]
    fn try_get_strong_count(&self) -> impl Try<Output = usize> {
        Option::Some(alloc::sync::Weak::strong_count(self))
    }

    #[inline]
    fn weak_count(&self) -> usize {
        alloc::sync::Weak::weak_count(self)
    }

    #[inline]
    fn try_upgrade(&self) -> impl Try<Output = Self::Upgraded> {
        alloc::sync::Weak::upgrade(self)
    }
}

#[cfg(feature = "rc")]
impl<T, A> TrBoxed for alloc::rc::Rc<T, A>
where
    T: ?Sized,
    A: core::alloc::Allocator,
{
    type Item = T;
    type Malloc = CoreAlloc;

    #[inline]
    fn try_get_mem_alloc(&self) -> impl Try<Output = &Self::Malloc> {
        Option::Some(CoreAlloc::shared())
    }
}

#[cfg(feature = "rc")]
impl<T, A> TrStrongShared for alloc::rc::Rc<T, A>
where
    T: ?Sized,
    A: core::alloc::AllocatorClone,
{
    type Downgraded = alloc::rc::Weak<T, A>;

    #[inline]
    fn strong_count(&self) -> usize {
        alloc::rc::Rc::strong_count(self)
    }

    #[inline]
    fn try_get_weak_count(&self) -> impl Try<Output = usize> {
        Option::Some(alloc::rc::Rc::weak_count(self))
    }

    #[inline]
    fn downgrade(&self) -> Self::Downgraded {
        alloc::rc::Rc::downgrade(self)
    }
}

#[cfg(feature = "rc")]
impl<T, A> TrWeakShared for alloc::rc::Weak<T, A>
where
    T: ?Sized,
    A: core::alloc::AllocatorClone,
{
    type Item = T;
    type Upgraded = alloc::rc::Rc<T, A>;

    #[inline]
    fn try_get_strong_count(&self) -> impl Try<Output = usize> {
        Option::Some(alloc::rc::Weak::strong_count(self))
    }

    #[inline]
    fn weak_count(&self) -> usize {
        alloc::rc::Weak::weak_count(self)
    }

    #[inline]
    fn try_upgrade(&self) -> impl Try<Output = Self::Upgraded> {
        alloc::rc::Weak::upgrade(self)
    }
}

#[cfg(feature = "box")]
impl<T, A> TrBoxed for alloc::boxed::Box<T, A>
where
    T: ?Sized,
    A: core::alloc::Allocator,
{
    type Item = T;
    type Malloc = CoreAlloc;

    fn try_get_mem_alloc(&self) -> impl Try<Output = &Self::Malloc> {
        Option::Some(CoreAlloc::shared())
    }
}

#[cfg(feature = "box")]
impl<T, A> TrUnique for alloc::boxed::Box<T, A>
where
    T: ?Sized,
    A: core::alloc::Allocator,
{}

#[cfg(test)]
mod tests_ {
    use core::ops::ControlFlow;

    #[allow(unused_imports)]
    use super::*;

    #[cfg(feature = "arc")]
    #[test]
    fn arc_should_impl_shared() {
        use alloc::sync::{Arc, Weak};

        let arc = Arc::new(());
        let weak = TrStrongShared::downgrade(&arc);
        assert_eq!(Arc::strong_count(&arc), TrStrongShared::strong_count(&arc));
        assert_eq!(Weak::weak_count(&weak), TrWeakShared::weak_count(&weak));

        let ControlFlow::Continue(upgraded) = TrWeakShared::try_upgrade(&weak).branch() else {
            panic!()
        };
        assert_eq!(Arc::strong_count(&arc), upgraded.strong_count());
        assert_eq!(upgraded.strong_count(), 2);
    }

    #[cfg(feature = "rc")]
    #[test]
    fn rc_should_impl_shared() {
        use alloc::rc::{Rc, Weak};

        let rc = Rc::new(());
        let weak = TrStrongShared::downgrade(&rc);
        assert_eq!(Rc::strong_count(&rc), TrStrongShared::strong_count(&rc));
        assert_eq!(Weak::weak_count(&weak), TrWeakShared::weak_count(&weak));

        let ControlFlow::Continue(upgraded) = TrWeakShared::try_upgrade(&weak).branch() else {
            panic!()
        };
        assert_eq!(Rc::strong_count(&rc), upgraded.strong_count());
        assert_eq!(upgraded.strong_count(), 2);
    }
}
