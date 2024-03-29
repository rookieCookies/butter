use std::mem::size_of;

use crate::ptr::WasmPtr;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct MemoryBlock {
    ptr: *mut (),
}

type Word = usize;

static mut ALLOC : Walloc = Walloc {
    free_lists: Vec::new(),
};


struct Walloc {
    free_lists: Vec<WasmPtr<Block>>,
}


#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Block {
    next: WasmPtr<Block>,
    used_n_size: usize,

    data: Word,
}


impl Block {
    fn size(&self) -> usize { 
        self.used_n_size & !1
    }

    fn is_used(&self) -> bool {
        (self.used_n_size & 1) != 0
    }

    fn set_used(ptr: *mut Self, b: bool) {
        if b {
            unsafe { (*ptr).used_n_size |= 1; }
        } else {
            unsafe { (*ptr).used_n_size &= !1; }
        }
    }
}


fn ptr_to_wptr<T>(memory: &mut impl Allocable, wasm_ptr: *const T) -> WasmPtr<T> {
    let ptr = wasm_ptr as usize;
    let dptr = memory.data_ptr() as usize;
    WasmPtr::<T>::from_u32((ptr - dptr) as u32)
}



///
/// Returns a `MemoryBlock` which contains the pointer
/// to a memory block at least `size` bytes.
/// 
/// The `size` field of the `MemoryBlock` indicates the
/// actual size of the memory allocated
///
pub fn walloc(memory: &mut impl Allocable, size: usize) -> WasmPtr<()> {
    println!("allocing {size}");
    let size = align_to(size, size_of::<Word>());
    
    // Search for a block
    if let Some(block) = find_block(memory, size) {
        let block = try_split(memory, block, size);
        Block::set_used(block.as_mut(memory), true);

        let data = unsafe { &mut (*block.as_mut(memory)).data } as *mut usize as *mut u8;
        return ptr_to_wptr(memory, data.cast())
    }

    // If not found, allocate
    let block = request_memory(memory, alloc_size(size)).expect("Out of memory");

    let block = block.as_mut(memory);
    unsafe {
        (*block).used_n_size = size;
        Block::set_used(block, true);
    }


    let data = unsafe { &mut (*block).data } as *mut usize as *mut u8;
    ptr_to_wptr(memory, data.cast())
}


///
/// Frees a previously allocated block
///
pub fn free(memory: &mut impl Allocable, ptr: WasmPtr<()>) {
    println!("freeing");
    let ptrb = get_header(ptr);

    let mut curr = ptrb;
    while curr.as_u32() != 0 {
        let currp = curr.as_ptr(memory);
        unsafe {
            if !(*currp).is_used() {
                let size = alloc_size((*currp).size());
                (*ptrb.as_mut(memory)).used_n_size += size;
                if curr.as_u32() as usize + size > memory.size() as usize { break };
                curr = WasmPtr::from_u32(curr.as_u32() + size as u32);
                continue
            }
        }
        break
    }

    Block::set_used(ptrb.as_mut(memory), false);
    let bucket = unsafe { get_bucket((*ptrb.as_ptr(memory)).size()) };
    unsafe { ALLOC.free_lists[bucket] = ptrb };
}


fn find_block(memory: &mut impl Allocable, size: usize) -> Option<WasmPtr<Block>> {
    let mut bucket = get_bucket(size);
    let len = unsafe { ALLOC.free_lists.len() };
    let mut left = len;
    while left != 0 {
        let mut curr = unsafe { ALLOC.free_lists[bucket] };
        left -= 1;

        while curr.as_u32() != 0 {
            let currp = curr.as_ptr(memory);

            unsafe {
                if (*currp).is_used() || (*currp).size() < size {
                    if (*currp).next.as_u32() == 0 { continue }
                    curr = (*currp).next;
                    continue;
                }


                ALLOC.free_lists[get_bucket(size)] = (*currp).next;
                return Some(curr);
            }
        }

        bucket = (bucket + 1) % len;
    }

    None
}


fn get_bucket(size: usize) -> usize {
    let bucket = size / size_of::<Word>() - 1;
    if unsafe { ALLOC.free_lists.len() } <= bucket {
        unsafe { ALLOC.free_lists.resize(bucket + 1, WasmPtr::from_u32(0)) }
    }

    bucket
}


fn get_header(ptr: WasmPtr<()>) -> WasmPtr<Block> {
    WasmPtr::from_u32(ptr.as_u32() + size_of::<Word>() as u32 - size_of::<Block>() as u32)
}


pub fn align_to(n: usize, alignment: usize) -> usize {
  return (n + alignment - 1) & !(alignment - 1);
}


fn alloc_size(size: usize) -> usize {
    size + size_of::<Block>() - size_of::<Word>()
}


fn try_split(memory: &mut impl Allocable, ptr: WasmPtr<Block>, size: usize) -> WasmPtr<Block> {
    unsafe {
        let ptrp = ptr.as_mut(memory);
        debug_assert!(!(*ptrp).is_used());

        if (*ptrp).size() > size + size_of::<Block>() {
            let nptr = ptrp.cast::<u8>().add(alloc_size(size));
            let nptr = nptr.cast::<Block>();
            let nsize = (*ptrp).size() - alloc_size(size);

            {
                let bucket = get_bucket(nsize);
                let ptr = ALLOC.free_lists[bucket];
                nptr.write(Block {
                    next: ptr, data: 0,
                    used_n_size: size,
                });

                ALLOC.free_lists[bucket] = ptr_to_wptr(memory, nptr);
            }

            (*ptrp).used_n_size = size;
        }
    }

    ptr
}

static mut PTR : usize = 0;

pub fn set_heap_start(ptr: WasmPtr<u8>) {
    unsafe { PTR = ptr.as_u32() as usize }
}

fn request_memory(memory: &mut impl Allocable, size: usize) -> Option<WasmPtr<Block>> {
    unsafe {
    if PTR as usize + size >= memory.data_size() {
        match memory.grow(1) {
            true => {
                return request_memory(memory, size)
            }
            false => panic!(),
        }
    }

    let ptr = WasmPtr::from_u32(PTR as u32);
    PTR += size;
    Some(ptr)
    }
}


pub trait Allocable {
    fn data_ptr(&self) -> *mut u8;
    fn data_size(&self) -> usize;
    fn size(&self) -> usize;
    fn grow(&mut self, delta: usize) -> bool;
}


struct MockMemory {
    vec: Vec<Word>,
}

impl MockMemory { 
    #[allow(unused)]
    fn new() -> Self { Self { vec: Vec::new() } }
}

const PAGE_SIZE : usize = 64 * 1024;
impl Allocable for MockMemory {
    fn data_ptr(&self) -> *mut u8 {
        self.vec.as_ptr().cast_mut().cast()
    }

    fn data_size(&self) -> usize {
        self.vec.len()
    }

    fn size(&self) -> usize {
        self.data_size() / PAGE_SIZE
    }

    fn grow(&mut self, delta: usize) -> bool {
        self.vec.resize(self.vec.len() + delta * PAGE_SIZE, 0);
        true
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align() {
        align_to(3, 8);  //  8
        align_to(8, 8);  //  8
        align_to(12, 8); // 16
        align_to(16, 8); // 16
         
        align_to(3, 4);  //  4
        align_to(8, 4);  //  8
        align_to(12, 4); // 12
        align_to(16, 4); // 16
    }


    #[test]
    fn test_alloc() {
        let mut mem = MockMemory::new();

        let ptr = walloc(&mut mem, 69); // 72
        let ptrb = get_header(ptr);
        {
            let block = unsafe { ptrb.as_ptr(&mut mem).read() };
            assert_eq!(block.size(), 72);
            assert!(block.is_used());
        }

        let ptr1 = walloc(&mut mem, 32); // 32 
        let ptr1b = get_header(ptr1);
        {
            let block = unsafe { ptr1b.as_ptr(&mut mem).read() };
            assert_eq!(block.size(), 32);
            assert!(block.is_used());
            dbg!(ptr1.as_u32());
            dbg!(ptr.as_u32());
            assert_eq!(ptr1.as_u32() as usize, ptr.as_u32() as usize + 72 + 16);
        }

        unsafe { *ptr.as_mut(&mut mem).cast::<usize>() = 69 };
        unsafe { *ptr1.as_mut(&mut mem).cast::<usize>() = 420 };

        let ptr2 = walloc(&mut mem, 12);
        assert_eq!(unsafe { ptr.as_ptr(&mut mem).cast::<usize>().read() }, 69);
        free(&mut mem, ptr);
        free(&mut mem, ptr2);

        assert_eq!(unsafe { ptr1.as_ptr(&mut mem).cast::<usize>().read() }, 420);
    }
}
