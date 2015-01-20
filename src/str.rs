use std::intrinsics;

pub trait StringTools {
    fn in_place<F>(&mut self, f: F)
            where F: FnOnce(&str) -> &str;
}

impl StringTools for String {
    fn in_place<F>(&mut self, f: F)
            where F: FnOnce(&str) -> &str {
        let (ptr2, len2);
        {
            let s2 = f(&**self);
            ptr2 = s2.as_ptr();
            len2 = s2.len();
            // Make sure `s2` is within `s`
            self.subslice_offset(s2);
        }
        let vec = unsafe { self.as_mut_vec() };
        unsafe {
            intrinsics::copy_memory(vec.as_mut_ptr(), ptr2, len2);
            vec.set_len(len2);
        }
    }
}

#[test]
fn in_place() {
    let mut s = "   hello world \n".to_string();
    let cap = s.capacity();
    s.in_place(|s| s.trim());
    assert_eq!(s, "hello world");
    assert_eq!(s.capacity(), cap);
}
