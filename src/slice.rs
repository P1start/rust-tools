use std::mem;

pub trait VecTools<T> {
    fn in_place<F>(&mut self, f: F)
            where F: FnOnce(&[T]) -> &[T];
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
            //offset = s2.as_ptr() as usize - self.as_ptr() as usize;
            len2 = s2.len();
        }
        for i in (0..len2) {
            self.swap(i, i + offset);
        }
        self.truncate(len2);
    }
}

#[test]
fn in_place() {
    let mut v = vec![0, 1, 2, 3, 4, 5, 6, 7];
    v.in_place(|x| &x[3..]);
    assert_eq!(v, [3, 4, 5, 6, 7]);
}
