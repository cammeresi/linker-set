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
    pub unsafe fn new(start: *const T, stop: *const T) -> Self {
        assert!(start < stop);
        let len = stop.offset_from(start).try_into().unwrap();
        let slice = std::slice::from_raw_parts(start, len);
        Self { start, stop, slice }
    }

    pub fn iter(&self) -> LinkerSetIter<T> {
        unsafe { LinkerSetIter::new(self.start, self.stop) }
    }

    pub fn len(&self) -> usize {
        self.slice.len()
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
macro_rules! set {
    ($set:ident) => {{
        paste::paste! {
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
