use std::collections::BTreeMap;

use windows::core::*;
// use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::IO::*;

fn main() {
    for i in 2..20 {
        let buffer_size = 1 << i;
        let elapsed = read_file(buffer_size);
        println!("buffer size: {}, time elapsed: {:?}", buffer_size, elapsed);
    }
}

fn read_file(buffer_size: usize) -> std::time::Duration {
    let file = unsafe {
        CreateFileW(
            w!("./test_file.txt"),
            FILE_GENERIC_READ,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )
        .expect("can't open file")
    };

    let mut overlapped = OVERLAPPED::default();

    let mut map = BTreeMap::new();
    let mut buffer = vec![0; buffer_size];

    let timer = std::time::Instant::now();

    loop {
        unsafe {
            _ = ReadFile(file, Some(&mut buffer), None, Some(&mut overlapped));
            WaitForSingleObject(overlapped.hEvent, u32::MAX);

            let mut bytes_copied = 0;
            let res = GetOverlappedResult(file, &overlapped, &mut bytes_copied, false);
            overlapped.Anonymous.Anonymous.Offset += bytes_copied;
            if !res.as_bool() {
                break;
            }
        }

        buffer.make_ascii_lowercase();

        for &byte in &buffer {
            let character = byte as char;

            if let Some(count) = map.get_mut(&character) {
                *count += 1;
            } else {
                map.insert(character, 1);
            }
        }
    }

    // println!("count of each character in text:");

    // for (character, count) in map {
    //     println!("{:?}: {}", character, count);
    // }

    timer.elapsed()
}
