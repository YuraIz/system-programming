use std::collections::BTreeMap;
use std::io::BufRead;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Mailslots::*;
use windows::Win32::System::Threading::*;

const EATING_TIME: std::time::Duration = std::time::Duration::from_millis(20);

fn main() {
    create_threads(5);
}

fn create_threads(thread_count: usize) {
    let mutexes: Vec<_> = vec![(); thread_count + 1]
        .iter()
        .map(|_| WinMutex::new())
        .collect();

    let mut philosophers = vec![];

    for i in 0..thread_count {
        philosophers.push(ThreadData {
            thread_num: i,
            forks: (i, (i + 1) % thread_count),
            mutexes: &mutexes,
        });
    }

    let mut handles = vec![];

    unsafe {
        for thread_data in philosophers.iter_mut() {
            let handle = CreateThread(
                None,
                0,
                Some(with_hierarchy),
                Some(thread_data.as_void_ptr()),
                THREAD_CREATION_FLAGS(0),
                None,
            )
            .expect("failed to create thread");

            handles.push(handle);
        }

        WaitForMultipleObjects(&handles, true, u32::MAX);

        for handle in handles {
            CloseHandle(handle);
        }
    }
}

struct WinMutex {
    handle: HANDLE,
}

impl WinMutex {
    fn new() -> Self {
        unsafe {
            Self {
                handle: CreateMutexW(None, false, None).expect("can't create mutex"),
            }
        }
    }

    #[must_use]
    fn lock(&self) -> WinMutexGuard {
        unsafe {
            WaitForSingleObject(self.handle, u32::MAX);
            WinMutexGuard(self.handle)
        }
    }
}

struct WinMutexGuard(HANDLE);

impl std::ops::Drop for WinMutexGuard {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0).expect("can't release mutex");
        }
    }
}

struct ThreadData<'a> {
    thread_num: usize,
    forks: (usize, usize),
    mutexes: &'a [WinMutex],
}

impl ThreadData<'_> {
    fn as_void_ptr(&mut self) -> *mut std::ffi::c_void {
        self as *mut _ as _
    }

    unsafe fn from_void_ptr(data: *mut std::ffi::c_void) -> &'static mut Self {
        &mut *(data as *mut ThreadData)
    }
}

unsafe extern "system" fn locking_algorhitm(data: *mut std::ffi::c_void) -> u32 {
    let data = ThreadData::from_void_ptr(data);

    let num = data.thread_num;

    let (left, right) = data.forks;
    let left = &data.mutexes[left];
    let right = &data.mutexes[right];

    loop {
        println!("Philosopher {} is thinking", num);
        {
            let l = left.lock();
            println!("Philosopher {} got left fork", num);
            let r = right.lock();
            println!("Philosopher {} eating", num);
            std::mem::drop((l, r));
            std::thread::sleep(EATING_TIME);
        }
    }
    0
}

unsafe extern "system" fn with_hierarchy(data: *mut std::ffi::c_void) -> u32 {
    let data = ThreadData::from_void_ptr(data);

    let num = data.thread_num;

    let (left, right) = data.forks;
    let (first, second) = if left > right {
        (left, right)
    } else {
        (right, left)
    };

    let first = &data.mutexes[first];
    let second = &data.mutexes[second];

    loop {
        println!("Philosopher {} is thinking", num);
        {
            let f = first.lock();
            println!("Philosopher {} got left fork", num);
            let s = second.lock();
            println!("Philosopher {} eating", num);
            std::mem::drop(s);
            std::mem::drop(f);
            std::thread::sleep(EATING_TIME);
        }
    }
    0
}

unsafe extern "system" fn with_arbitrator(data: *mut std::ffi::c_void) -> u32 {
    let data = ThreadData::from_void_ptr(data);

    let num = data.thread_num;

    let (left, right) = data.forks;
    let left = &data.mutexes[left];
    let right = &data.mutexes[right];

    unsafe fn take_both_forks(
        first: &WinMutex,
        second: &WinMutex,
    ) -> (WinMutexGuard, WinMutexGuard) {
        WaitForMultipleObjects(&[first.handle, second.handle], true, u32::MAX);
        return (WinMutexGuard(first.handle), WinMutexGuard(second.handle));
    }

    loop {
        println!("Philosopher {} is thinking", num);
        {
            let (l, r) = take_both_forks(left, right);
            println!("Philosopher {} eating", num);
            std::mem::drop((l, r));
            std::thread::sleep(EATING_TIME);
        }
    }
    0
}
