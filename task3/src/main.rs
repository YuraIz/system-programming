use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::{
    CreateThread, SetThreadPriority, WaitForMultipleObjects, THREAD_CREATION_FLAGS,
    THREAD_PRIORITY_ABOVE_NORMAL, THREAD_PRIORITY_BELOW_NORMAL, THREAD_PRIORITY_HIGHEST,
    THREAD_PRIORITY_LOWEST,
};

fn main() {
    unsafe { spawn_threads(8) }
}

struct ThreadData {
    thread_num: usize,
    thread_priority: usize,
    items: Vec<u32>,
}

unsafe fn spawn_threads(thread_count: usize) {
    let mut items = vec![];

    for i in 0..thread_count {
        items.push(ThreadData {
            thread_num: i,
            thread_priority: i % 4,
            items: (0..10_000_000).rev().collect(),
        });
    }

    let mut handles = vec![];

    for thread_data in items.iter_mut() {
        let priority = match thread_data.thread_priority {
            0 => THREAD_PRIORITY_LOWEST,
            1 => THREAD_PRIORITY_BELOW_NORMAL,
            2 => THREAD_PRIORITY_ABOVE_NORMAL,
            3 => THREAD_PRIORITY_HIGHEST,
            _ => unreachable!(),
        };

        let handle = CreateThread(
            None,
            0,
            Some(work),
            Some(thread_data as *mut _ as _),
            THREAD_CREATION_FLAGS(0),
            None,
        )
        .expect("failed to create thread");

        SetThreadPriority(handle, priority);

        handles.push(handle);
    }

    WaitForMultipleObjects(&handles, true, u32::MAX);

    for handle in handles {
        CloseHandle(handle);
    }

    for item in items {
        dbg!(item.items.len());
    }
}

unsafe extern "system" fn work(data: *mut std::ffi::c_void) -> u32 {
    let data = &mut *(data as *mut ThreadData);

    let timer = std::time::Instant::now();

    println!(
        "Thread {} with priority {} started",
        data.thread_num, data.thread_priority
    );

    data.items.sort_unstable();

    println!(
        "Thread {} with priority {} finished in {:?}",
        data.thread_num,
        data.thread_priority,
        timer.elapsed()
    );

    0
}
