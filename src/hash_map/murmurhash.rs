pub fn calculate(key: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xcc9e2d51;
    const C2: u32 = 0x1b873593;
    const R1: u32 = 15;
    const R2: u32 = 13;
    const M: u32 = 5;
    const N: u32 = 0xe6546b64;

    let len = u32::try_from(key.len()).expect("key is too large");
    let key = key.as_ptr();

    let mut k;
    let l = (len / 4) as i32;

    let mut h = seed;

    unsafe {
        let chunks: *const u32 = key.add((l * 4) as usize).cast(); // body
        let tail: *const u8 = key.add((l * 4) as usize).cast(); // last 8 byte chunk of `key'

        let mut i = -l;
        while i != 0 {
            k = chunks.offset(i as isize).read_unaligned();

            k = k.wrapping_mul(C1);
            k = k << R1 | k >> (32 - R1);
            k = k.wrapping_mul(C2);

            h ^= k;
            h = h << R2 | h >> (32 - R2);
            h = h.wrapping_mul(M).wrapping_add(N);

            i += 1;
        }

        k = 0;

        let rem = len & 3;
        if rem >= 3 {
            k ^= (tail.add(2).read() as u32) << 16;
        }
        if rem >= 2 {
            k ^= (tail.add(1).read() as u32) << 8;
        }
        if rem >= 1 {
            k ^= tail.add(0).read() as u32;
            k = k.wrapping_mul(C1);
            k = k << R1 | k >> (32 - R1);
            k = k.wrapping_mul(C2);
            h ^= k;
        }
    }

    h ^= len;

    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;

    h
}
