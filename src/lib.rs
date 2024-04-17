pub use linker_set_proc::set_entry;

pub struct LinkerSetIter<T> {
    next: *const T,
    stop: *const T,
}

impl<T> LinkerSetIter<T> {
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

#[macro_export]
macro_rules! set_declare {
    ($set:ident, $type:ty) => {
        pub mod $set {
            #[allow(unused_imports)]
            use super::*;
            paste::paste! {
                extern {
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

#[macro_export]
macro_rules! set_iter {
    ($set:ident) => {{
        paste::paste! {
            unsafe {
                LinkerSetIter::new(
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
    fn test_set_iter() {
        let actual = set_iter!(stuff).collect::<HashSet<_>>();
        let expect = HashSet::from([&FOO, &BAR, &0x6666666666666666]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_set_len() {
        const LEN: usize = 3;
        let iter = set_iter!(stuff);
        assert_eq!(iter.len(), LEN);
        assert_eq!(iter.size_hint(), (LEN, Some(LEN)));
        assert_eq!(iter.count(), LEN);
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
        let actual = set_iter!(aaa).collect::<HashSet<_>>();
        let expect = HashSet::from([&AAA]);
        assert_eq!(actual, expect);
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
        let iter = set_iter!(stuff);
        assert_eq!(iter.len(), LEN);
    }
}
