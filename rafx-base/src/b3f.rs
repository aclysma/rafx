// Basic Binary Block Format (B3F)
//
// File Format
// [4] magic number encoded as u32 (0xBB33FF00)
// [4] file tag (arbitrary 4 bytes for user)
// [4] version (arbitrary meaning for user, encoded as u32)
// [4] block count (encoded as u32)
// [8] bytes indicating 0 (0x00)
// [8*n] ending offset of block
// [x] pad to 16 byte offset
// [n*len(n)] data (format/encoding/semantics would be implied by file tag). Each block begins at
// [x] pad to 16 byte offset
//
// Endianness is undefined. Use the magic number to detect if endianness is different between
// writer/reader
//
// This format can be encoded into a block, making this structure hierarchical. In this
// case, omit the magic number, and use the file tag to optionally indicate the contents
// of the block. (So it becomes a "block tag")
//
// if you c-cast the range memory from byte 16 to block count * 4, you have an array of u32 of n+1
// length where n is number of blocks. Offset for block n is given by array[n]. End of block n is
// given by array[n+1]. Size of block n in bytes is given by array[n+1] - array[n]
//
// Alignment of blocks to 16 bytes promotes reinterpreting bytes i.e. u8 to u64 or __m128 without
// tripping over undefined behavior

use std::convert::TryInto;

const HEADER_SIZE_IN_BYTES: usize = 16;
const BLOCK_LENGTH_SIZE_IN_BYTES: usize = 8;
const BLOCK_ALIGNMENT_IN_BYTES: usize = 16;

pub struct B3FReader<'a> {
    data: &'a [u8],
}

impl<'a> B3FReader<'a> {
    pub fn new(data: &'a [u8]) -> Option<B3FReader<'a>> {
        if data.len() < 16 {
            return None;
        }

        let magic_number = u32::from_ne_bytes(data[0..4].try_into().ok()?);
        if magic_number != 0xBB33FF00 {
            return None;
        }

        let reader = B3FReader { data };

        Some(reader)
    }

    pub fn file_tag_as_u32(&self) -> u32 {
        u32::from_ne_bytes(self.data[4..8].try_into().unwrap())
    }

    pub fn file_tag_as_u8(&self) -> &[u8] {
        &self.data[4..8]
    }

    pub fn version(&self) -> u32 {
        u32::from_ne_bytes(self.data[8..12].try_into().unwrap())
    }

    pub fn block_count(&self) -> usize {
        u32::from_ne_bytes(self.data[12..16].try_into().unwrap()) as usize
    }

    pub fn get_block(
        &self,
        index: usize,
    ) -> &'a [u8] {
        // assumed by some implementation details here
        debug_assert_eq!(BLOCK_LENGTH_SIZE_IN_BYTES, 8);
        let begin_size_offset = HEADER_SIZE_IN_BYTES + (index * BLOCK_LENGTH_SIZE_IN_BYTES);
        let size_data = &self.data[begin_size_offset..];
        let mut begin = u64::from_ne_bytes(size_data[0..8].try_into().unwrap()) as usize;
        let end = u64::from_ne_bytes(size_data[8..16].try_into().unwrap()) as usize;

        // Begin position needs to be rounded up to 16-byte offset
        begin = ((begin + BLOCK_ALIGNMENT_IN_BYTES - 1) / BLOCK_ALIGNMENT_IN_BYTES)
            * BLOCK_ALIGNMENT_IN_BYTES;

        let mut data_offset =
            HEADER_SIZE_IN_BYTES + ((self.block_count() + 1) * BLOCK_LENGTH_SIZE_IN_BYTES);
        data_offset = ((data_offset + BLOCK_ALIGNMENT_IN_BYTES - 1) / BLOCK_ALIGNMENT_IN_BYTES)
            * BLOCK_ALIGNMENT_IN_BYTES;
        &self.data[data_offset..][begin..end]
    }
}
