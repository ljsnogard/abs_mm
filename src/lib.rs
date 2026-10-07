#![no_std]

#![feature(allocator_ext)]
// #![feature(layout_for_ptr)]  // this has been stable
// #![feature(ptr_as_uninit)]
// #![feature(ptr_as_ref_unchecked)]
// #![feature(slice_ptr_get)]
#![feature(try_trait_v2)]

// We always pull in `std` during tests, because it's just easier
// to write tests when you can assume you're on a capable platform
#[cfg(test)]
extern crate std;

#[cfg(any(test, feature = "core_alloc"))]
mod core_alloc_;

pub mod res_man;

#[cfg(any(test, feature = "core_alloc"))]
pub use core_alloc_::CoreAlloc;
