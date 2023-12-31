use std::marker::PhantomData;

use sti::{prelude::Arena, arena::ArenaStats, hash::{HashFn, fxhash::FxHasher32, HashMapF}, define_key};

define_key!(u32, pub StringIndex);

pub struct StringMap<'a> {
    arena: &'a Arena,
    map: sti::hash::HashMapF<HashStr<'a>, StringIndex, HashStrHashFn>,
    // map: FuckMap<&'static str, StringIndex>,
    vec: Vec<&'a str>,
}


impl<'a> StringMap<'a> {
    pub const INIT_FUNC : StringIndex = StringIndex(0);
    pub const TRUE : StringIndex = StringIndex(1);
    pub const FALSE : StringIndex = StringIndex(2);
    pub const VALUE : StringIndex = StringIndex(3);
    pub const BOOL : StringIndex = StringIndex(4);
    pub const STR : StringIndex = StringIndex(5);
    pub const INT : StringIndex = StringIndex(6);
    pub const FLOAT : StringIndex = StringIndex(7);
    pub const ANY : StringIndex = StringIndex(8);
    pub const OK : StringIndex = StringIndex(9);
    pub const ERR : StringIndex = StringIndex(10);
    pub const SOME : StringIndex = StringIndex(11);
    pub const NONE : StringIndex = StringIndex(12);
 
    #[inline(always)]
    pub fn new(arena: &'a Arena) -> Self {
        Self::with_capacity(0, arena)
    }

    
    #[inline(always)]
    pub fn with_capacity(cap: usize, arena: &'a Arena) -> Self {
        let mut s = Self {
            map: HashMapF::fwith_cap(cap),
            // map: FuckMap::with_capacity(cap),
            vec: Vec::with_capacity(cap),
            arena,
        };

        assert_eq!(s.insert("_init"), Self::INIT_FUNC);
        assert_eq!(s.insert("true"), Self::TRUE);
        assert_eq!(s.insert("false"), Self::FALSE);
        assert_eq!(s.insert("value"), Self::VALUE);
        assert_eq!(s.insert("bool"), Self::BOOL);
        assert_eq!(s.insert("str"), Self::STR);
        assert_eq!(s.insert("int"), Self::INT);
        assert_eq!(s.insert("float"), Self::FLOAT);
        assert_eq!(s.insert("any"), Self::ANY);
        assert_eq!(s.insert("ok"), Self::OK);
        assert_eq!(s.insert("err"), Self::ERR);
        assert_eq!(s.insert("some"), Self::SOME);
        assert_eq!(s.insert("none"), Self::NONE);
        s
    }


    #[inline(always)]
    pub fn insert(&mut self, value: &str) -> StringIndex {
        let key = HashStr::new(unsafe { std::mem::transmute(value) });
        *self.map.get_or_insert_with_key(&key, |_| {
            debug_assert!(self.vec.len() < u32::MAX as usize);

            let alloc = self.arena.alloc_str(value);

            let index = StringIndex(self.vec.len() as u32);
            self.vec.push(alloc);

            (HashStr::with_hash(alloc, key.hash()), index)
        })
    }


    #[inline(always)]
    pub fn get(&self, index: StringIndex) -> &'a str {
        &self.vec[index.0 as usize]
    }


    #[inline(always)]
    pub fn len(&self) -> usize { self.vec.len() }


    #[inline(always)]
    pub fn capacity(&self) -> usize { self.vec.capacity().min(self.map.cap()) }


    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.vec.is_empty() }


    #[inline(always)]
    pub fn arena_stats(&self) -> ArenaStats {
        self.arena.stats()
    }


    #[inline(always)]
    pub fn reserve(&mut self, additional: usize) {
        self.vec.reserve(additional);
        self.map.reserve(additional);
    }
}


impl std::fmt::Debug for StringMap<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.vec.iter().enumerate()).finish()
    }
}


impl PartialEq for StringMap<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.vec == other.vec
    }
}


impl Drop for StringMap<'_> {
    fn drop(&mut self) {
        // self.map.clear();
    }
}


#[derive(Clone, Copy)]
pub struct HashStr<'a> {
    ptr: *const u8,
    len: u32,
    hash: u32,
    phantom: PhantomData<&'a str>,
}


impl<'a> HashStr<'a> {
    pub fn new(str: &'a str) -> Self {
        let ptr = str.as_ptr();
        let len = str.len().try_into().unwrap();
        let hash = FxHasher32::hash_bytes(str.as_bytes());
        Self { ptr, len, hash, phantom: PhantomData }
    }

    
    #[inline(always)]
    pub fn with_hash(str: &'a str, hash: u32) -> Self {
        let ptr = str.as_ptr();
        let len = str.len().try_into().unwrap();
        Self { ptr, len, hash, phantom: PhantomData }
    }


    #[inline(always)]
    pub fn hash(&self) -> u32 { self.hash }


    #[inline(always)]
    pub fn as_str(&self) -> &'a str {
        unsafe {
            core::str::from_utf8_unchecked(
                core::slice::from_raw_parts(
                    self.ptr, self.len as usize))
        }
    }
}


impl<'a> core::ops::Deref for HashStr<'a> {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { self.as_str() }
}


impl<'a> PartialEq for HashStr<'a> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.as_str() == other.as_str()
    }
}


impl<'a> Eq for HashStr<'a> {}


struct HashStrHashFn;

impl<'a> HashFn<HashStr<'a>> for HashStrHashFn {
    type Seed = ();
    type Hash = u32;

    const DEFAULT_SEED: () = ();

    #[inline(always)]
    fn hash_with_seed(_: (), value: &HashStr<'a>) -> u32 {
        value.hash
    }
}
