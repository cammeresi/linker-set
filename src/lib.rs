#![warn(missing_docs)]

//! Declarative programming via embedded configuration data
//!
//! A linker set allows you to program declaratively rather than
//! imperatively by embedding configuration or behavior into a program as
//! data.
//!
//! Using a linker set, you can scatter instances of a certain type all
//! over your program, and, with the proper annotations, the linker will
//! gather them up into a special section of the ELF binary, forming an
//! array, which can be iterated at runtime.
//!
//! # Example
//!
//! ```
//! use std::collections::HashSet;
//! use linker_set::*;
//!
//! set_declare!(stuff, u64);
//!
//! #[set_entry(stuff)]
//! static FOO: u64 = 0x4F202A76B86A7299u64;
//! #[set_entry(stuff)]
//! static BAR: u64 = 0x560E9309456ACCE0u64;
//!
//! # fn main() {
//! let actual = set!(stuff).iter().collect::<HashSet<_>>();
//! let expect = HashSet::from([&FOO, &BAR]);
//! assert_eq!(actual, expect);
//! # }
//! ```
//!
//! The [set_declare!] macro outputs a module definition.  The module must
//! be imported into the scope of calls to the [set_entry] attribute and the
//! [set!] macro.
//!
//! If you make a linker set of an integer type, you should use typed
//! literals, not generic integer literals.  I.e.
//!
//! ```
//! use linker_set::*;
//!
//! set_declare!(foo, u64);
//!
//! #[set_entry(foo)]
//! static FOO: u64 = 1000u64; // not 1000 ❌
//! ```
//!
//! Generic integer literals will defeat a typechecking mechanism that is
//! output by the [set_entry] macro.
//!
//! All items in a set should be of the same size, the size of the declared
//! type.  Otherwise, stuff won't work.  The macros make an attempt to
//! typecheck set entries, but they aren't foolproof.  Caveat scriptor.
//!
//! The index operator is kind of just for fun.  Obviously you shouldn't
//! depend on the linker to provide any specific ordering.
//!
//! # Safety
//!
//! Although the [set_entry] macro does not require an unsafe to call, it is
//! not entirely safe.  The caller is required to ensure all entries in the
//! set are valid and in the proper format.  Rust may add unsafe macros at
//! some point, but at present there is no way to declare that a given
//! third-party macro is unsafe, even though Rust 2024 has some attributes
//! that require an unsafe to be used.
//!
//! # Compatibility
//!
//! This crate works on Linux x86-64.  It may work on other similar (i.e.
//! ELF-based) targets.
//!
//! # History
//!
//! This idea comes from [Clustrix], the best distributed relational
//! database in the world, which no one knew about.  Clustrix was written
//! in a very unusual but very interesting style of C.  Much of it was
//! written in [continuation-passing style]([CPS]), and continuations and
//! lightweight threads (fibers) ran on top of a scheduler very similar to
//! the asynchronous runtimes like Tokio which later became popular.  (But
//! Clustrix was started in 2006, before that popularity.)
//!
//! Linker sets were used extensively in the Clustrix code to do things
//! such as specify initialization or other system processes via graphs
//! (initgraphs), automatically create heaps for memory allocation,
//! automatically allocate integers or flags for what would otherwise have
//! to be centrally controlled constants, and automatically register
//! structures or handlers with a subsystem.
//!
//! This concept was present in the oldest version of the Clustrix code in
//! Git.  A prior Subversion repository seemed to have been lost.  The
//! inspiration appears to have come from [FreeBSD], which has several
//! macros whose names match exactly macros used in the Clustrix source
//! code.
//!
//! [Clustrix]: https://en.wikipedia.org/wiki/Clustrix
//! [CPS]: https://en.wikipedia.org/wiki/Continuation-passing_style
//! [FreeBSD]: https://github.com/freebsd/freebsd-src/blob/main/sys/sys/linker_set.h

pub use linker_set_proc::set_entry;
pub use paste::paste;

/// An iterator that yields the elements in a linker set.
pub struct LinkerSetIter<T> {
    next: *const T,
    stop: *const T,
}

impl<T> LinkerSetIter<T> {
    /// Create a new iterator for a linker set.
    ///
    /// Users should call the [set!] macro instead of this function.
    ///
    /// # Safety
    /// The pointers must be start and end pointers generated by the linker.
    pub unsafe fn new(start: *const T, stop: *const T) -> Self {
        assert!(start < stop);
        Self { next: start, stop }
    }
}

impl<T> Iterator for LinkerSetIter<T>
where
    T: 'static,
{
    type Item = &'static T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.stop {
            None
        } else {
            unsafe {
                let x = self.next.as_ref();
                self.next = self.next.add(1);
                x
            }
        }
    }

    fn count(self) -> usize {
        self.len()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T> ExactSizeIterator for LinkerSetIter<T>
where
    T: 'static,
{
    fn len(&self) -> usize {
        unsafe { self.stop.offset_from(self.next).try_into().unwrap() }
    }
}

impl<T> std::iter::FusedIterator for LinkerSetIter<T> where T: 'static {}

unsafe impl<T: Send> Send for LinkerSetIter<T> {}

/// A proxy object that represents a linker set.
///
/// You can store this object if you should want the ability to create
/// multiple iterators on the linker set, or maybe if you wanted to keep
/// track of a specific linker set out of some number of them.
pub struct LinkerSet<T>
where
    T: 'static,
{
    start: *const T,
    stop: *const T,
    slice: &'static [T],
}

impl<T> LinkerSet<T>
where
    T: 'static,
{
    /// Create a new object to represent a linker set.
    ///
    /// # Safety
    /// The pointers must be start and end pointers generated by the linker.
    pub unsafe fn new(start: *const T, stop: *const T) -> Self {
        assert!(start < stop);
        let slice = unsafe {
            let len = stop.offset_from(start).try_into().unwrap();
            std::slice::from_raw_parts(start, len)
        };
        Self { start, stop, slice }
    }

    /// Returns an iterator over the items in the linker set.
    pub fn iter(&self) -> LinkerSetIter<T> {
        unsafe { LinkerSetIter::new(self.start, self.stop) }
    }

    /// Returns the number of elements in the linker set.
    pub fn len(&self) -> usize {
        self.slice.len()
    }

    /// Returns true if the linker set contains zero elements.
    pub fn is_empty(&self) -> bool {
        self.start == self.stop
    }
}

impl<T> IntoIterator for LinkerSet<T>
where
    T: 'static,
{
    type Item = &'static T;
    type IntoIter = LinkerSetIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T, I> std::ops::Index<I> for LinkerSet<T>
where
    T: 'static,
    I: std::slice::SliceIndex<[T], Output = T>,
{
    type Output = T;

    fn index(&self, i: I) -> &Self::Output {
        self.slice.index(i)
    }
}

unsafe impl<T: Send> Send for LinkerSet<T> {}
unsafe impl<T: Sync> Sync for LinkerSet<T> {} // readonly once created

/// Declare the name of a linker set.
///
/// This macro outputs a module into the current scope.  The module must
/// be brought into scope should the linker set be used within another module.
#[macro_export]
macro_rules! set_declare {
    ($set:ident, $type:ty) => {
        pub mod $set {
            #[allow(unused_imports)]
            use super::*;
            $crate::paste! {
                unsafe extern {
                    /* rust thinks we're allowing these things to come in from
                     * C code, so if type is a function, it gets cranky because
                     * it thinks we're proposing to call a function in C with
                     * rust calling convention. */
                    #[allow(improper_ctypes)]
                    pub static [<__start_set_ $set>]: $type;
                    #[allow(improper_ctypes)]
                    pub static [<__stop_set_ $set>]: $type;
                }
            }
        }
    };
}

/// Create a linker set proxy object for iteration or indexing.
#[macro_export]
macro_rules! set {
    ($set:ident) => {{
        $crate::paste! {
            unsafe {
                LinkerSet::new(
                    &$set::[<__start_set_ $set>],
                    &$set::[<__stop_set_ $set>],
                )
            }
        }
    }};
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    set_declare!(stuff, u64);

    #[set_entry(stuff)]
    static FOO: u64 = 0x4F202A76B86A7299u64;
    #[set_entry(stuff)]
    static BAR: u64 = 0x560E9309456ACCE0u64;

    #[test]
    fn test_set_contents() {
        let actual = set!(stuff).iter().collect::<HashSet<_>>();
        let expect = HashSet::from([&FOO, &BAR, &0x6666666666666666]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_set_iter_len() {
        const LEN: usize = 3;
        let iter = set!(stuff).iter();
        assert_eq!(iter.len(), LEN);
        assert_eq!(iter.size_hint(), (LEN, Some(LEN)));
        assert_eq!(iter.count(), LEN);
    }

    #[test]
    fn test_into() {
        let mut actual = HashSet::new();
        for i in set!(stuff) {
            actual.insert(i);
        }
        let expect = HashSet::from([&FOO, &BAR, &0x6666666666666666]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_index() {
        let set = set!(stuff);
        assert_eq!(set.len(), 3);
        let mut actual = HashSet::new();
        for i in 0..set.len() {
            actual.insert(set[i]); // this is u64; compiler auto derefs
        }
        let expect = HashSet::from([FOO, BAR, 0x6666666666666666]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_is_empty() {
        assert!(!set!(stuff).is_empty());
    }

    #[derive(Debug, Eq, PartialEq, Hash)]
    pub(crate) struct Foo {
        a: u32,
        b: u8,
    }

    set_declare!(aaa, Foo);

    #[set_entry(aaa)]
    static AAA: Foo = Foo { a: 1, b: 5 };

    #[test]
    fn test_struct() {
        let actual = set!(aaa).iter().collect::<HashSet<_>>();
        let expect = HashSet::from([&AAA]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_traits() {
        fn require_send<T: Send>(_: T) {}
        fn require_sync<T: Sync>(_: T) {}

        require_send(set!(aaa));
        require_sync(set!(aaa));
        require_send(set!(aaa).iter());
    }
}

#[cfg(test)]
mod test_use_ext {
    use super::*;
    use test::stuff;

    #[set_entry(stuff)]
    static FOO: u64 = 0x6666666666666666;

    #[test]
    fn test_use() {
        const LEN: usize = 3;
        let iter = set!(stuff).iter();
        assert_eq!(iter.len(), LEN);
    }
}
