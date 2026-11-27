const MAX_SCAN_SIZE: usize = 1024;

static MAGIC_NUMBERS: [&[u8]; 2] = [b"%PDF", b"\x89PNG"];

pub fn is_binary(buffer: &[u8]) -> bool {
    let scan_size = buffer.len().min(MAX_SCAN_SIZE);

    if buffer[..scan_size].contains(&0) {
        return true;
    }

    MAGIC_NUMBERS.iter().any(|magic| buffer.starts_with(magic))
}
