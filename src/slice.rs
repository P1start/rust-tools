use std::mem;

pub trait VecTools<T> {
    fn in_place<F>(&mut self, f: F)
            where F: FnOnce(&[T]) -> &[T];
}

pub trait SliceTools<T> {
    /// Promotes a tuple of non-overlapping slices into `self` into a tuple of mutable slices into
    /// `self`.
    ///
    /// # Panics
    ///
    /// Panics if the slices returned by `f` intersect each other at all.
    fn promote<F>(&mut self, f: F) -> (&mut [T], &mut [T])
            where F: FnOnce(&[T]) -> (&[T], &[T]);

    /// Removes an element from the slice and return it and the remaining section of the slice,
    /// swapping the removed element with the first in the slice.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    fn swap_remove(&mut self, index: usize) -> (&mut T, &mut [T]);

    /// Returns a streaming iterator over mutable references to the items of `self` and the ‘rests’
    /// of the slice (using `swap_remove` to separate the items from the rests).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use tools::slice::SliceTools;
    /// # use tools::iter::StreamingIterator;
    ///
    /// let mut a = [0, 1, 2, 3];
    /// let mut it = a.remove_iter();
    /// while let Some((item, rest)) = it.next_streaming() {
    ///     println!("{}, {:?}", item, rest);
    /// }
    /// // Prints:
    /// // 0, [3, 1, 2]
    /// // 1, [0, 3, 2]
    /// // 2, [0, 1, 3]
    /// // 3, [0, 1, 2]
    /// ```
    fn remove_iter(&mut self) -> RemoveIter<T>;
}

fn subslice_offset<T>(slf: &[T], inner: &[T]) -> usize {
    let a_start = slf.as_ptr() as usize;
    let a_end = a_start + slf.len() * mem::size_of::<T>();
    let b_start = inner.as_ptr() as usize;
    let b_end = b_start + inner.len() * mem::size_of::<T>();

    assert!(a_start <= b_start);
    assert!(b_end <= a_end);
    (b_start - a_start) / mem::size_of::<T>()
}

impl<T> VecTools<T> for Vec<T> {
    fn in_place<F>(&mut self, f: F)
            where F: FnOnce(&[T]) -> &[T] {
        let (offset, len2);
        {
            let s2 = f(&**self);
            // Make sure `s2` is within `self`
            offset = subslice_offset(&**self, s2);
            len2 = s2.len();
        }
        for i in (0..len2) {
            self.swap(i, i + offset);
        }
        self.truncate(len2);
    }
}

impl<T> SliceTools<T> for [T] {
    fn promote<F>(&mut self, f: F) -> (&mut [T], &mut [T])
            where F: FnOnce(&[T]) -> (&[T], &[T]) {
        let (mut a, mut b) = f(&*self);
        if a.as_ptr() > b.as_ptr() { mem::swap(&mut a, &mut b); }
        assert!(a.as_ptr() >= self.as_ptr());
        assert!(b.as_ptr() as usize >= a.as_ptr() as usize + a.len()*mem::size_of::<T>());
        assert!(b.as_ptr() as usize + b.len()*mem::size_of::<T>()
             <= self.as_ptr() as usize + self.len()*mem::size_of::<T>());
        unsafe {
            mem::transmute((a, b))
        }
    }

    fn swap_remove(&mut self, idx: usize) -> (&mut T, &mut [T]) {
        let len = self.len();
        assert!(len > 0, "SliceTools::swap_remove called on slice of length 0");
        self.swap(len - 1, idx);
        let (rest, last) = self.split_at_mut(len - 1);
        let last = &mut last[0];
        (last, rest)
    }

    fn remove_iter(&mut self) -> RemoveIter<T> {
        RemoveIter {
            slice: self,
            idx: 0,
        }
    }
}

pub struct RemoveIter<'a, T: 'a> {
    slice: &'a mut [T],
    idx: usize,
}

impl<'a, 'b, T> ::iter::StreamingIterator<'a> for RemoveIter<'b, T> {
    type Item = (&'a mut T, &'a mut [T]);

    fn next_streaming(&'a mut self) -> Option<(&'a mut T, &'a mut [T])> {
        let len = self.slice.len();
        if self.idx > 0 {
            // Undo the swap_remove from the previous iteration
            self.slice.swap(self.idx - 1, len - 1);
        }
        self.idx += 1;
        if self.idx > len { return None }
        Some(self.slice.swap_remove(self.idx - 1))
    }
}

#[test]
fn test_in_place() {
    let mut v = vec![0, 1, 2, 3, 4, 5, 6, 7];
    let (ptr, cap) = (v.as_ptr(), v.capacity());
    v.in_place(|x| &x[3..]);
    assert_eq!(v, [3, 4, 5, 6, 7]);
    assert_eq!(v.as_ptr(), ptr);
    assert_eq!(v.capacity(), cap);
}

#[test]
fn test_promote() {
    let mut v = vec![0, 1, 2, 3, 4, 5, 6, 7];
    let (a, b) = v.promote(|x| {
        x.split_at(3)
    });
    assert_eq!(a, [0, 1, 2]);
    assert_eq!(b, [3, 4, 5, 6, 7]);
}

#[test]
fn test_swap_remove() {
    let mut a = [0, 1, 2, 3, 4, 5];
    let (two, rest) = a.swap_remove(2);
    assert_eq!(*two, 2);
    assert_eq!(rest, [0, 1, 5, 3, 4]);
}

#[test]
fn test_rest_iter() {
    use ::iter::StreamingIterator;

    let mut a = [0, 1, 2, 3];
    {
        let mut it = a.remove_iter();
        while let Some((i, rest)) = it.next_streaming() {
            assert_eq!(rest, match *i {
                0 => [3, 1, 2],
                1 => [0, 3, 2],
                2 => [0, 1, 3],
                3 => [0, 1, 2],
                _ => unreachable!(),
            });
        }
    }
    assert_eq!(a, [0, 1, 2, 3]);
}
