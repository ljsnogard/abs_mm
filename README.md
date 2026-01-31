# abs_mm

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ljsnogard/abs_mm)

Abstract Memory Management. 

Need nightly rustc and unstable feature. Check (lib.rs)[./src/lib.rs] for detail.

Mod `mem_alloc` provides trait `TrMalloc` as an alternatives to unstable `Allocator` trait in core for memory allocation.  
Mod `typed_alloc` provides trait `TrTypedAlloc`
Mod `res_man` provides a serie of traits abstracting smart pointers like `core::sync::Arc` and `core::boxed::Box`.  
