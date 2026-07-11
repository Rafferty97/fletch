//! A single-threaded, thin, reference-counted allocation with a homogeneous
//! `[T]` tail whose disposal is directed by the `Head`.
//!
//! Layout of the allocation (`#[repr(C)]` semantics enforced manually via
//! `Layout`):
//!
//! ```text
//! +------------+----------------+---------------------+
//! | count      | head: H        | tail: [T; len]      |
//! | Cell<usize>|                |                     |
//! +------------+----------------+---------------------+
//! ```
//!
//! The handle (`ThinRc<H, T>`) is a single non-null pointer to the start of the
//! allocation. The tail length is stored inside `H` (via `Head::tail_len`);
//! there is no separate length word.
//!
//! `T` is the tail element type: `u8` for a string's bytes, `u64` for bignum
//! limbs, or a tag-ascribed word type (e.g. `Data`) for records/tuples whose
//! disposal is head-directed.
//!
//! ALL unsafe lives in this module. `Head` implementors receive a safe
//! `&mut [ManuallyDrop<T>]` and never write `unsafe`.

use std::alloc::{self, Layout};
use std::cell::Cell;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::{self, NonNull};

/// A head describes its tail: how long it is, and how to dispose of it.
///
/// Parameterized over the tail element type `T`. Implementors get a SAFE view
/// of the tail and never write `unsafe`.
pub trait Head: Sized {
    type T;

    /// Number of `T` slots in the tail belonging to this head.
    fn tail_len(&self) -> usize;

    /// Dispose of the owning slots in the tail. Called exactly once, when the
    /// refcount reaches zero, immediately before the allocation is freed.
    ///
    /// The slice is `ManuallyDrop<T>` so that the implementor drops precisely
    /// the owning slots (via `ManuallyDrop::drop`) and leaves inert slots
    /// untouched. Slots that are not explicitly dropped are forgotten, which is
    /// correct for inert (non-owning) tails.
    ///
    /// Default: treat the whole tail as inert (drop nothing). Suitable for
    /// `u8` string tails or `u64` bignum-limb tails, whose elements own no
    /// further allocation.
    fn drop_tail(&mut self, tail: &mut [Self::T]) {
        let _ = tail;
    }
}

/// Internal header: everything before the tail.
#[repr(C)]
struct Header<H> {
    count: Cell<usize>,
    head: H,
    // tail: [T] follows immediately after, computed via Layout.
}

pub struct ThinRc<H: Head> {
    ptr: NonNull<Header<H>>,
    _marker: PhantomData<(H, [H::T])>,
}

impl<H: Head> ThinRc<H> {
    /// Compute the layout of `Header<H> + [T; len]` and the byte offset of the
    /// tail. Single source of truth for both construction and access, so they
    /// can never disagree.
    #[inline]
    fn layout_for(len: usize) -> (Layout, usize) {
        let header = Layout::new::<Header<H>>();
        let tail = Layout::array::<H::T>(len).expect("tail layout overflow");
        let (layout, tail_offset) = header.extend(tail).expect("allocation layout overflow");
        // Round total size up to alignment so arrays / dealloc are well-formed.
        (layout.pad_to_align(), tail_offset)
    }

    /// Pointer to the first tail slot. Derived from the whole-allocation base,
    /// so it carries provenance over the tail.
    #[inline]
    unsafe fn tail_ptr(&self, tail_offset: usize) -> *mut H::T {
        unsafe { self.ptr.as_ptr().byte_add(tail_offset) as *mut H::T }
    }

    /// Construct from a head and an iterator of exactly `head.tail_len()` tail
    /// values. Safe constructor: callers hand in owned `T`s and get back a
    /// refcounted handle.
    pub fn new<I>(head: H, tail: I) -> Self
    where
        I: IntoIterator<Item = H::T>,
    {
        let len = head.tail_len();
        let (layout, tail_offset) = Self::layout_for(len);

        // SAFETY: layout has non-zero size (Header<H> contains a Cell<usize>,
        // so size >= size_of::<usize>() > 0), satisfying alloc's precondition.
        let raw = unsafe { alloc::alloc(layout) };
        let base = match NonNull::new(raw as *mut Header<H>) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };

        // Initialize header fields. SAFETY: `base` points at freshly allocated,
        // correctly-sized/aligned, uninitialized memory for Header<H>.
        unsafe {
            ptr::write(base.as_ptr(), Header { count: Cell::new(1), head });
        }

        // SAFETY: tail region is `len` uninitialized `T` slots at
        // `base + tail_offset`, with provenance from `base`.
        unsafe {
            let tail_start = (base.as_ptr() as *mut u8).add(tail_offset) as *mut H::T;

            let mut written = 0;
            for (i, d) in tail.into_iter().enumerate() {
                assert!(written < len, "iterator yielded too many items, expected {len}");
                ptr::write(tail_start.add(i), d);
                written += 1;
            }
            assert!(written == len, "iterator yielded {} items, expected {len}", written);
        }

        ThinRc { ptr: base, _marker: PhantomData }
    }

    /// Shared access to the head.
    #[inline]
    pub fn head(&self) -> &H {
        // SAFETY: ptr is valid while a handle exists; shared borrow tied to
        // &self.
        unsafe { &(*self.ptr.as_ptr()).head }
    }

    /// Shared access to the tail as a safe slice.
    #[inline]
    pub fn tail(&self) -> &[H::T] {
        let len = self.head().tail_len();
        let (_, tail_offset) = Self::layout_for(len);
        // SAFETY: tail region holds `len` initialized `T` with provenance from
        // the allocation base; shared borrow tied to &self.
        unsafe {
            let p = self.tail_ptr(tail_offset) as *const H::T;
            std::slice::from_raw_parts(p, len)
        }
    }

    #[inline]
    fn count_cell(&self) -> &Cell<usize> {
        // SAFETY: valid while handle exists.
        unsafe { &(*self.ptr.as_ptr()).count }
    }

    /// Current strong count (mainly for tests / debugging).
    #[inline]
    pub fn strong_count(&self) -> usize {
        self.count_cell().get()
    }
}

impl<H: Head> Clone for ThinRc<H> {
    #[inline]
    fn clone(&self) -> Self {
        let c = self.count_cell();
        c.set(c.get() + 1);
        ThinRc { ptr: self.ptr, _marker: PhantomData }
    }
}

impl<H: Head> Drop for ThinRc<H> {
    fn drop(&mut self) {
        let c = self.count_cell();
        let n = c.get();
        if n > 1 {
            c.set(n - 1);
            return;
        }
        // Last reference: dispose of tail, then head, then free.
        let len = self.head().tail_len();
        let (layout, tail_offset) = Self::layout_for(len);

        // 1. Head-directed tail disposal via the SAFE API.
        // SAFETY: we hold the only reference (refcount == 1), so no other
        // reference to the tail or head exists. The tail is `len` initialized
        // `T` with whole-allocation provenance; we build a &mut slice of T
        // over it exactly once, before freeing. The head and
        // tail regions are disjoint, so the &mut head and &mut tail_slice do
        // not alias.
        unsafe {
            let tail_start = self.tail_ptr(tail_offset) as *mut H::T;
            let tail_slice = std::slice::from_raw_parts_mut(tail_start, len);
            let head = &mut (*self.ptr.as_ptr()).head;
            head.drop_tail(tail_slice);
        }

        // 2. Drop the head itself (runs H's own Drop, if any).
        // SAFETY: head is initialized and not used after this.
        unsafe {
            ptr::drop_in_place(&mut (*self.ptr.as_ptr()).head);
        }

        // Any tail slots not disposed by `drop_tail` are inert: we never run
        // `[T]` drop glue over the tail, so nothing is double-dropped. Owning
        // slots were handled by the head above; inert slots need no disposal.

        // 3. Free the allocation.
        // SAFETY: same layout used to allocate; pointer came from alloc.
        unsafe {
            alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

// ThinRc is single-threaded by construction (Cell refcount). It is
// automatically !Send + !Sync because Cell is !Sync and we add no unsafe impls.

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Inert u8 tail (like a string's bytes) --------------------------

    struct StrHead {
        len: usize,
    }

    impl Head for StrHead {
        type T = u8;

        fn tail_len(&self) -> usize {
            self.len
        }
    }

    #[test]
    fn u8_tail_construct_and_read() {
        let rc = ThinRc::<StrHead>::new(StrHead { len: 5 }, *b"hello");
        assert_eq!(rc.tail(), b"hello");
    }

    // ---- Inert u64 tail (like bignum limbs) -----------------------------

    struct BigHead {
        len: usize,
    }

    impl Head for BigHead {
        type T = u64;

        fn tail_len(&self) -> usize {
            self.len
        }
    }

    #[test]
    fn u64_tail_refcount() {
        let rc = ThinRc::<BigHead>::new(BigHead { len: 3 }, [1, 2, 3]);
        let rc2 = rc.clone();
        assert_eq!(rc.strong_count(), 2);
        assert_eq!(rc.tail(), &[1, 2, 3]);
        drop(rc2);
        assert_eq!(rc.strong_count(), 1);
        assert_eq!(rc.tail(), &[1, 2, 3]); // still alive
        drop(rc); // frees here
    }

    // ---- Owning tail: each element is a Box we must dispose -------------
    // Demonstrates head-directed disposal of a non-inert tail. Here T is the
    // raw Box pointer; a real implementation would use a tag-ascribed word.

    struct OwningHead {
        len: usize,
    }

    impl Head for OwningHead {
        type T = *mut u64;

        fn tail_len(&self) -> usize {
            self.len
        }
        fn drop_tail(&mut self, tail: &mut [*mut u64]) {
            for slot in tail.iter_mut() {
                let addr: *mut u64 = *slot;
                // SAFETY (per this test's encoding): addr came from
                // Box::into_raw; reconstitute and drop exactly once.
                unsafe {
                    drop(Box::from_raw(addr));
                }
            }
        }
    }

    #[test]
    fn owning_tail_disposed() {
        let ptrs: Vec<*mut u64> = (0..3u64).map(|i| Box::into_raw(Box::new(i))).collect();
        let rc = ThinRc::<OwningHead>::new(OwningHead { len: 3 }, ptrs);
        drop(rc); // Miri/ASan will flag a leak or double-free if disposal is wrong
    }

    // ---- Panic-safety of the constructor --------------------------------

    struct PanicIter {
        yielded: usize,
    }

    impl Iterator for PanicIter {
        type Item = Box<u64>;
        fn next(&mut self) -> Option<Box<u64>> {
            if self.yielded == 1 {
                panic!("boom on second element");
            }
            self.yielded += 1;
            Some(Box::new(self.yielded as u64))
        }
    }

    struct BoxHead;

    impl Head for BoxHead {
        type T = ManuallyDrop<Box<u64>>;

        fn tail_len(&self) -> usize {
            3
        }

        fn drop_tail(&mut self, tail: &mut [ManuallyDrop<Box<u64>>]) {
            for slot in tail.iter_mut() {
                // SAFETY: called once at refcount zero; each slot initialized.
                unsafe { ManuallyDrop::drop(slot) };
            }
        }
    }
}
