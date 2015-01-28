use std::mem;

pub trait VecTools<T> {
    fn in_place<F>(&mut self, f: F)
            where F: FnOnce(&[T]) -> &[T];
}
pub trait SliceTools<T> {
    /// Promote a vector of slices into `self` into a vector of mutable slices into `self`.
    ///
    /// # Panic
    ///
    /// Panics if the slices returned by `f` intersect each other at all.
    fn promote<F>(&mut self, f: F) -> (&mut [T], &mut [T])
            where F: FnOnce(&[T]) -> (&[T], &[T]);
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
