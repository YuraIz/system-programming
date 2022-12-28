use std::collections::BTreeMap;
use std::io::BufRead;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Mailslots::*;

const SLOT_NAME: &HSTRING = w!(r#"\\.\mailslot\sample_mailslot"#);

fn main() {
    if let Some(username) = std::env::args().nth(1) {
        user_loop(username);
    } else {
        server_loop();
    }
}

fn user_loop(username: String) {
    let file = unsafe {
        CreateFileW(
            SLOT_NAME,
            FILE_GENERIC_WRITE,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .expect("can't open file")
    };

    let userslotname = format!(r#"\\.\mailslot\user_slots\{}"#, username);

    let userslot = unsafe {
        CreateMailslotW(&HSTRING::from(&userslotname), 0, 0, None).expect("can't create mailslot")
    };

    let regi_string = format!("regi:{}:{}", username, userslotname);
    unsafe {
        WriteFile(file, Some(regi_string.as_bytes()), None, None);
    }

    std::thread::spawn(move || unsafe {
        let mut next_size: u32 = 0;
        let mut message_count: u32 = 0;

        loop {
            GetMailslotInfo(
                userslot,
                None,
                Some(&mut next_size),
                Some(&mut message_count),
                None,
            )
            .expect("can't get mailslot info");

            if message_count != 0 {
                let mut buffer = vec![0; next_size as usize];

                _ = ReadFile(userslot, Some(&mut buffer), None, None);
                println!("{}", String::from_utf8_lossy(&buffer));
            }
        }
    });

    let mut stdin = std::io::stdin().lock();
    let mut buf = String::new();
    loop {
        stdin.read_line(&mut buf).unwrap();
        if buf == "exit\r\n" {
            break;
        } else {
            let msg_string = format!("text:{}:{}", username, buf);
            unsafe {
                WriteFile(file, Some(msg_string.as_bytes()), None, None);
            }
        }
        buf.clear();
    }
}

fn server_loop() {
    let slot = unsafe { CreateMailslotW(SLOT_NAME, 0, 0, None).expect("can't create mailslot") };

    let mut user_slots = BTreeMap::new();

    unsafe {
        loop {
            let messages = Message::fetch(slot);

            if !messages.is_empty() {
                for message in messages {
                    match message {
                        Message::Register { username, slotname } => {
                            let hstring = HSTRING::from(&slotname);
                            let slotfile = CreateFileW(
                                &hstring,
                                FILE_GENERIC_WRITE,
                                FILE_SHARE_READ,
                                None,
                                OPEN_EXISTING,
                                FILE_ATTRIBUTE_NORMAL,
                                None,
                            )
                            .expect("can't open user slot");
                            println!("{} registered", username);
                            user_slots.insert(username, slotfile);
                        }
                        Message::Text { from, to, text } => {
                            println!("text from {} to {:?}: {}", from, to, text);
                            let composed_message = format!("{}:{}", from, text);
                            for receiver in to {
                                if let Some(file) = user_slots.get(&receiver) {
                                    WriteFile(
                                        file.to_owned(),
                                        Some(composed_message.as_bytes()),
                                        None,
                                        None,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_secs_f32(0.5));
        }
    }
}

#[derive(Debug)]
enum Message {
    Register {
        username: String,
        slotname: String,
    },
    Text {
        from: String,
        to: Vec<String>,
        text: String,
    },
}

impl Message {
    fn fetch(slot: HANDLE) -> Vec<Self> {
        let mut messages = vec![];

        let mut next_size: u32 = 0;
        let mut message_count: u32 = 0;

        unsafe {
            loop {
                GetMailslotInfo(
                    slot,
                    None,
                    Some(&mut next_size),
                    Some(&mut message_count),
                    None,
                )
                .expect("can't get mailslot info");

                if message_count == 0 {
                    break;
                }

                let mut buffer = vec![0; next_size as usize];

                _ = ReadFile(slot, Some(&mut buffer), None, None);

                let string = String::from_utf8(buffer).unwrap();

                match &string[..4] {
                    "regi" => {
                        let mut splitted = string.split(':');
                        _ = splitted.next();
                        let username = splitted.next().unwrap().to_string();
                        let slotname = splitted.next().unwrap().to_string();
                        messages.push(Self::Register { username, slotname });
                    }
                    "text" => {
                        let mut splitted = string.split(':');
                        _ = splitted.next();
                        let from = splitted.next().unwrap().to_string();
                        let to = splitted
                            .next()
                            .unwrap()
                            .to_string()
                            .split(',')
                            .map(|s| s.to_string())
                            .collect();
                        let text_parts: Vec<_> = splitted.collect();
                        let text = text_parts.join(":");
                        messages.push(Self::Text { from, to, text });
                    }
                    _ => unreachable!(),
                }
            }
        }

        messages
    }
}
