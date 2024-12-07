//! A Rust crate for a generational map, handle map, whatever you want to
//! call it.  Whatever it is, this is a random-access data structure that
//! stores an unordered bag of items and gives you a handle for each
//! specific item you insert.  Looking things up by handle is `O(1)` --
//! the backing storage is just a `Vec` -- and items can be removed as
//! well, which is also `O(1)`.  Handles are small (two `usize`'s) and
//! easy to copy, similar to a slice.  The trick is that there is a
//! generation number stored with each item, so that a "dangling"
//! handle that refers to an item that has been removed is invalid.
//! Unlike array indices, if you remove an item from the array and a new
//! one gets put in its place, the old stale handle does not refer to the
//! new one and trying to use it will fail at runtime.
//!
//! This is useful for various things: managing lots of uniform things
//! with shared ownership (such as video game resources), interning
//! strings, that sort of thing.  Essentially by using handles to items
//! you have runtime-checked memory safety: you can get an item from the
//! map using the handle, since it's basically just an array index.
//!
//! In practice this is not unrelated to `Rc`, it's just that `Rc` does
//! the accounting of memory on cloning the `Rc`, and this does it on
//! "dereferencing" by looking up an object's handle.  Referencing
//! counting, threading garbage collection and this sort of map are all
//! different methods of achieving the same thing (checking for memory
//! safety at runtime) with different tradeoffs.  With a reference count
//! you don't have to explicitly free items and can't have stale handles,
//! but loops require special handling and you pay for the accounting cost
//! on cloning a handle.  With this you have to free items explicitly, but
//! stale handles can be safely detected and the accounting happens when
//! you look the item up.  You can also use it as a slab allocator type
//! thing, where you pre-allocate a large amount of storage and free it
//! all at once.

/// A small, easy-to-copy handle referring to a location in
/// a particular `GenMap`.
///
/// Handles from one `GenMap` are not valid to use in a different
/// `GenMap`, and this can *not* be detected at runtime.  It is recommended
/// to wrap handles in a newtype struct to make sure at compile-time
/// that you have the right one.  Support for this may become built in
/// to the API, but for the moment it's unclear how to do it best.
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Handle {
    pub gen: usize,
    pub idx: usize,
}

// TODO: IntoIterator and such???

/// Iterator over keys in a `GenMap`.
#[derive(Debug, Clone)]
pub struct Iterator<'a, T> {
    i: std::iter::Enumerate<std::slice::Iter<'a, (usize, Slot<T>)>>,
}

/// The contents of a slot in a `GenMap`.
#[derive(Debug, Clone)]
pub enum Slot<T> {
    /// Just the item
    Occupied { itm: T },
    /// The location of the next free slot in the freelist.
    // TODO: Maybe NonZeroUsize?  Meh.
    Empty { next_free: Option<usize> },
}

/// A collection of `T`'s referred to by `Handle`'s.
/// When you add an object to the `GenMap` it will
/// return a `Handle`, and you can look that item up
/// by that `Handle`.  You can also remove the item,
/// which makes any old `Handle`'s to it become invalid
/// and attempting to get it will return `None`.
#[derive(Debug, Clone, Default)]
pub struct GenMap<T> {
    /// The usize is the generation number.
    slots: Vec<(usize, Slot<T>)>,
    /// None means no free slots
    freelist_head: Option<usize>,
    /// Number of elements
    count: usize,
}

impl<T> GenMap<T> {
    /// Create a new empty `GenMap` with enough memory to accomodate
    /// the given number of items without reallocating.
    pub fn with_capacity(capacity: usize) -> Self {
        GenMap {
            slots: Vec::with_capacity(capacity),
            freelist_head: None,
            count: 0,
        }
    }

    
    pub fn inner_unck(&self) -> &Vec<(usize, Slot<T>)> {
        &self.slots
    }

    
    pub fn inner_unck_mut(&mut self) -> &mut Vec<(usize, Slot<T>)> {
        &mut self.slots
    }


    pub fn set_freelist_head(&mut self, val: Option<usize>) {
        self.freelist_head = val;
    }


    /// Insert the element into the map and return a handle referring to it.
    pub fn insert(&mut self, itm: T) -> Handle {
        self.count = self
            .count
            .checked_add(1)
            .expect("Count overflow; I bet this is a bug.");
        if let Some(i) = self.freelist_head {
            let slot = self
                .slots
                .get_mut(i)
                .expect("Invalid freelist head? Should never happen!");
            let gen = match slot {
                (_gen, Slot::Occupied { .. }) => {
                    unreachable!("Freelist points at an occupied slot, should never happen!");
                }
                (gen, Slot::Empty { next_free }) => {
                    self.freelist_head = *next_free;
                    gen
                }
            };
            let new_gen = gen.checked_add(1).expect("Aiee, generation overflowed!");
            *slot = (new_gen, Slot::Occupied { itm });
            Handle {
                gen: new_gen,
                idx: i,
            }
        } else {
            // Freelist is empty, we just create a new slot
            let idx = self.slots.len();
            let gen = 1;
            self.slots.push((gen, Slot::Occupied { itm }));
            Handle { gen, idx }
        }
    }

    /// Returns a reference to the item if the handle is valid,
    /// or `None` otherwise.
    pub fn get(&self, h: Handle) -> Option<&T> {
        match self.slots.get(h.idx) {
            None => None,
            Some((_, Slot::Empty { .. })) => None,
            Some((gen, Slot::Occupied { .. })) if *gen != h.gen => None,
            Some((_gen, Slot::Occupied { itm })) => Some(itm),
        }
    }

    /// Returns a mutable reference to the item if the handle is valid,
    /// or `None` otherwise.
    pub fn get_mut(&mut self, h: Handle) -> Option<&mut T> {
        match self.slots.get_mut(h.idx) {
            None => None,
            Some((_, Slot::Empty { .. })) => None,
            Some((gen, Slot::Occupied { .. })) if *gen != h.gen => None,
            Some((_gen, Slot::Occupied { itm })) => Some(itm),
        }
    }

    /// Removes the referenced item from the map, returning it.
    /// Returns None if the handle is stale.
    pub fn remove(&mut self, h: Handle) -> Option<T> {
        let s = self.slots.get_mut(h.idx);
        let slot_contents = match s {
            None => return None,
            Some((_gen, Slot::Empty { .. })) => return None,
            Some((gen, Slot::Occupied { .. })) if *gen != h.gen => return None,
            Some(t) => t,
        };

        self.count = self
            .count
            .checked_sub(1)
            .expect("Count underflow; should never happen");
        // The handle is not stale, yay, so now we remove its
        // contents and replace it with `Slot::Empty`.
        let gen = slot_contents.0;
        let new_slot = (
            gen,
            Slot::Empty {
                next_free: self.freelist_head,
            },
        );
        let old_contents = std::mem::replace(slot_contents, new_slot);
        self.freelist_head = Some(h.idx);

        // The only thing old_contents can be at this point is a
        // Slot::Occupied, hopefully the compiler can prove it?
        // TODO: We can make this better by pulling the contents out in the
        // first match.
        match old_contents {
            (_, Slot::Occupied { itm }) => Some(itm),
            _ => unreachable!("A slot magically went from occupied to empty!"),
        }
    }

    /// Number of items in the map.
    pub fn count(&self) -> usize {
        self.count
    }

    /// The total number of items the map has free memory to contain.
    pub fn capacity(&self) -> usize {
        self.slots.capacity()
    }

    pub fn iter(&self) -> Iterator<T> {
        Iterator {
            i: self.slots.iter().enumerate(),
        }
    }

    /// The number of items in the free list.
    /// Only for unit testing.
    #[allow(dead_code)]
    pub(crate) fn freelist_len(&self) -> usize {
        let mut len = 0;
        let mut head = self.freelist_head;
        while let Some(i) = head {
            len += 1;
            match self.slots[i] {
                (_gen, Slot::Empty { next_free }) => {
                    head = next_free;
                }
                _ => panic!("Freelist contains pointer to non-free slot?"),
            }
        }
        len
    }
}

impl<T> std::iter::Iterator for Iterator<'_, T> {
    type Item = Handle;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.i.next() {
                Some((_, (_, Slot::Empty { .. }))) => {
                    // Skip empty slots.
                    continue;
                }
                Some((idx, (gen, Slot::Occupied { .. }))) => {
                    return Some(Handle { idx, gen: *gen });
                }
                None => {
                    return None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let mut m: GenMap<String> = GenMap::default();
        let v1 = "thing1".to_owned();
        let v2 = "thing2".to_owned();
        let h1 = m.insert(v1.clone());
        let h2 = m.insert(v2.clone());
        assert_eq!(m.count(), 2);

        assert_eq!(&v1, m.get(h1).unwrap());
        assert_eq!(&v2, m.get(h2).unwrap());

        assert_ne!(&v1, m.get(h2).unwrap());
        assert_ne!(&v2, m.get(h1).unwrap());
    }

    #[test]
    fn test_remove() {
        let mut m: GenMap<String> = GenMap::default();
        let v1 = "thing1".to_owned();
        let v2 = "thing2".to_owned();
        let h1 = m.insert(v1.clone());
        let h2 = m.insert(v2.clone());
        assert_eq!(m.count(), 2);

        assert_eq!(&v1, m.get(h1).unwrap());
        assert_eq!(&v2, m.get(h2).unwrap());

        m.remove(h1);
        assert!(m.get(h1).is_none());
        assert!(m.get(h2).is_some());
        assert_eq!(m.count(), 1);

        m.remove(h2);
        assert!(m.get(h1).is_none());
        assert!(m.get(h2).is_none());
        assert_eq!(m.count(), 0);
    }

    #[test]
    fn test_remove_then_add() {
        let mut m: GenMap<String> = GenMap::default();
        let v1 = "thing1".to_owned();
        let v2 = "thing2".to_owned();
        let h1 = m.insert(v1.clone());
        let h2 = m.insert(v2.clone());

        assert_eq!(&v1, m.get(h1).unwrap());
        assert_eq!(&v2, m.get(h2).unwrap());

        m.remove(h1);
        assert_eq!(m.count(), 1);
        assert!(m.get(h1).is_none());
        assert!(m.get(h2).is_some());

        let v3 = "thing3".to_owned();
        let h3 = m.insert(v3.clone());
        assert!(m.get(h1).is_none());
        assert!(m.get(h3).is_some());
        assert_eq!(m.count(), 2);
    }

    #[test]
    fn test_freelist() {
        let mut m: GenMap<String> = GenMap::default();
        let v1 = "thing1".to_owned();
        let v2 = "thing2".to_owned();
        let v3 = "thing3".to_owned();
        let h1 = m.insert(v1.clone());
        let h2 = m.insert(v2.clone());
        let _h3 = m.insert(v3.clone());

        assert_eq!(&v1, m.get(h1).unwrap());
        assert_eq!(&v2, m.get(h2).unwrap());

        assert_eq!(m.freelist_len(), 0);
        assert_eq!(m.count(), 3);

        m.remove(h1);
        assert_eq!(m.freelist_len(), 1);
        assert_eq!(m.count(), 2);

        // Trying to remove same element = no change
        m.remove(h1);
        assert_eq!(m.freelist_len(), 1);
        assert_eq!(m.count(), 2);

        m.remove(h2);
        assert_eq!(m.freelist_len(), 2);
        assert_eq!(m.count(), 1);

        let h4 = m.insert("thing4".to_owned());
        assert_eq!(m.freelist_len(), 1);
        assert_eq!(m.count(), 2);

        m.remove(h4);
        assert_eq!(m.freelist_len(), 2);
        assert_eq!(m.count(), 1);
    }
}
