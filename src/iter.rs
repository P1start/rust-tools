use std::mem;
use std::iter::Peekable;
use arena::TypedArena;

// Infinite <3s to Luqman for most of this impl
pub struct Utf8Iter<I> where I: Iterator<Item = u8> {
    buf: Peekable<I>,
}

impl<I> Iterator for Utf8Iter<I>
        where I: Iterator<Item = u8> {

    type Item = Option<char>;

    fn next(&mut self) -> Option<Option<char>> {
        // Our mask to get the actual values
        // from continuation bytes
        const CONT_MASK: u8 = (1 << 6) - 1;

        if let Some(first) = self.buf.next() {
            // Single byte rune (ASCII)
            if (first & (1 << 7)) == 0 {
                return Some(Some(first as char));
            }

            // How many bytes make up this rune?
            let l = (!first).leading_zeros();

            // Grab the second
            let second = match self.buf.peek() {
                Some(&second) => second,
                None => return Some(None),
            };

            // Next, let's make sure we actually have valid input
            match (l, first, second) {
                // Two byte sequence, make sure it's
                // a continuation byte
                (2, _, _) => if (second & 0xC0) != 0x80 { return Some(None) },

                // Three byte sequence
                (3, 0xE0         , 0xA0 ... 0xBF) |
                (3, 0xE1 ... 0xEC, 0x80 ... 0xBF) |
                (3, 0xED         , 0x80 ... 0x9F) |
                (3, 0xEE ... 0xEF, 0x80 ... 0xBF) => {}

                // Four byte sequence
                (4, 0xF0         , 0x90 ... 0xBF) |
                (4, 0xF1 ... 0xF3, 0x80 ... 0xBF) |
                (4, 0xF4         , 0x80 ... 0x8F) => {}

                // Anything else is invalid
                _ => return Some(None)
            }
            self.buf.next();

            // Now let's create our value from the
            // first and second bytes by keeping the
            // bottom 3-5 bits as necessary from
            // the start byte and the bottom six
            // from the second.
            let mut result = (first & ((1 << (7 - l)) - 1)) as u32;
            result = result << 6 | (second & CONT_MASK) as u32;

            // Grab the rest of the bytes, if necessary
            for _ in 0 .. l-2 {
                if let Some(&next) = self.buf.peek() {
                    // Make sure this is a continuation byte
                    if (next & 0xC0) != 0x80 {
                        return Some(None);
                    }
                    self.buf.next();

                    // Tack on the bottom six bits onto our final result,
                    // shifting over the previous values
                    result = result << 6 | (next & CONT_MASK) as u32;
                } else { return Some(None); }
            }

            return Some(Some(unsafe { mem::transmute(result) }));

        } else {
            return None;
        }
    }
}

pub trait IterTools: Sized {
    fn utf8_iter(self) -> Utf8Iter<Self>
        where Self: Iterator<Item=u8>;

    fn group<F, G>(self, f: F) -> Groups<Self, F, G>
        where Self: Iterator, F: FnMut(&<Self as Iterator>::Item) -> G, G: PartialEq;

    fn refs(self) -> RefIter<Self>
        where Self: Iterator;

    fn dedup(self) -> DedupIter<Self>
        where Self: Iterator, <Self as Iterator>::Item: PartialEq;
}

impl<T> IterTools for T {
    #[inline(always)]
    fn utf8_iter(self) -> Utf8Iter<Self>
            where Self: Iterator<Item=u8> {
        Utf8Iter { buf: self.peekable() }
    }

    #[inline(always)]
    fn group<F, G>(self, f: F) -> Groups<Self, F, G>
            where Self: Iterator, F: FnMut(&<Self as Iterator>::Item) -> G, G: PartialEq {
        Groups {
            iter: self.peekable(),
            f: f,
            done: true,
        }
    }

    #[inline(always)]
    fn refs(self) -> RefIter<Self>
            where Self: Iterator {
        RefIter {
            iter: self,
            arena: TypedArena::new(),
        }
    }

    #[inline(always)]
    fn dedup(self) -> DedupIter<Self>
            where Self: Iterator, <Self as Iterator>::Item: PartialEq {
        DedupIter {
            iter: self.peekable(),
        }
    }
}

pub trait StreamingIterator<'a> {
    type Item;

    fn next_streaming(&'a mut self) -> Option<Self::Item>;
}

impl<'a, I> StreamingIterator<'a> for I
        where I: Iterator {
    type Item = <Self as Iterator>::Item;

    fn next_streaming(&'a mut self) -> Option<<Self as Iterator>::Item> {
        self.next()
    }
}

pub struct Groups<I, F, G>
        where I: Iterator, F: FnMut(&<I as Iterator>::Item) -> G, G: PartialEq {
    iter: Peekable<I>,
    f: F,
    done: bool,
}

impl<'a, I, F, G> StreamingIterator<'a> for Groups<I, F, G>
        where I: Iterator, F: FnMut(&<I as Iterator>::Item) -> G, G: PartialEq {
    type Item = (G, Group<'a, I, F, G>);

    fn next_streaming(&'a mut self) -> Option<(G, Group<'a, I, F, G>)> {
        let (mut g, mut g2);
        {
            {
                let p = self.iter.peek();
                if let Some(p) = p {
                    g = (self.f)(p);
                    g2 = (self.f)(p);
                } else { return None; }
            }
            if !self.done {
                loop {
                    {
                        let np = self.iter.peek();
                        if let Some(ng) = np {
                            if (self.f)(ng) != g {
                                g = (self.f)(ng);
                                g2 = (self.f)(ng);
                                break;
                            }
                        } else { return None; }
                    }
                    self.iter.next();
                }
            }
        }
        self.done = false;
        Some((g, Group {
            sup: self,
            g: g2,
        }))
    }
}

pub struct Group<'a, I: 'a, F: 'a, G>
        where I: Iterator, F: FnMut(&<I as Iterator>::Item) -> G, G: PartialEq, I::Item: 'a {
    sup: &'a mut Groups<I, F, G>,
    g: G,
}

impl<'a, I, F, G> Iterator for Group<'a, I, F, G>
        where I: Iterator, F: FnMut(&<I as Iterator>::Item) -> G, G: PartialEq, I::Item: 'a {
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        {
            let g = self.sup.iter.peek();
            if let Some(g2) = g {
                let g2 = (self.sup.f)(g2);
                if g2 != self.g {
                    self.sup.done = true;
                    return None;
                }
            } else {
                self.sup.done = true;
                return None;
            }
        }
        let a = self.sup.iter.next().unwrap();
        return Some(a);
    }
}

pub struct RefIter<I>
        where I: Iterator {
    iter: I,
    arena: TypedArena<<I as Iterator>::Item>,
}

impl<'a, I> StreamingIterator<'a> for RefIter<I>
        where I: Iterator {
    type Item = &'a mut <I as Iterator>::Item;

    #[inline]
    fn next_streaming(&'a mut self) -> Option<&'a mut <I as Iterator>::Item> {
        let a;
        match self.iter.next() {
            None => return None,
            Some(x) => a = x,
        }
        Some(self.arena.alloc(a))
    }
}

pub struct DedupIter<I>
        where I: Iterator, <I as Iterator>::Item: PartialEq {
    iter: Peekable<I>,
}

impl<I> Iterator for DedupIter<I>
        where I: Iterator, <I as Iterator>::Item: PartialEq {
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        let n = self.iter.next();
        if let None = n { return None; }
        while self.iter.peek() == n.as_ref() {
            self.iter.next();
        }
        n
    }
}

#[test]
fn utf8_chars() {
    // Single byte (ASCII): Latin Capital Letter B
    assert_eq!(vec![0x42].into_iter().utf8_iter().next(), Some(Some('B')));

    // Two Bytes: Latin Small Letter Gamma
    assert_eq!(vec![0xC9, 0xA3].into_iter().utf8_iter().next(), Some(Some('ɣ')));

    // Three Bytes: Snowman ☃
    assert_eq!(vec![0xE2, 0x98, 0x83].into_iter().utf8_iter().next(), Some(Some('☃')));

    // Four Bytes: Unicode Han Character 'to peel, pare'
    assert_eq!(vec![0xF0, 0xA0, 0x9C, 0xB1].into_iter().utf8_iter().next(), Some(Some('𠜱')));

    // Multiple runes
    assert_eq!(
        vec![0x42, 0xC9, 0xA3, 0xE2, 0x98, 0x83, 0xF0, 0xA0, 0x9C, 0xB1, 0xff, 0x41].into_iter().utf8_iter()
            .collect::<Vec<_>>(),
        vec![Some('B'), Some('ɣ'), Some('☃'), Some('𠜱'), None, Some('A')]
    );
}

#[test]
fn refs() {
    // Check lifetime stuff
    let mut iter = {
        let iter = 0i32..5;
        iter.refs()
    };
    let mut i = 0;
    while let Some(v) = iter.next_streaming() {
        assert_eq!(&mut i, v);
        i += 1;
    }
}

#[test]
fn dedup() {
    use std::vec::Vec;
    let mut v = vec![1, 1, 2, 3, 3, 4, 3, 3, 3, 3, 4, 4, 5, 6];
    Vec::dedup(&mut v);
    let v2: Vec<i32> = v.clone().into_iter().dedup().collect();
    assert_eq!(v, v2);
}
