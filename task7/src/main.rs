use std::collections::BTreeMap;
use std::io::BufRead;

use windows::core::*;
use windows::Win32::Networking::WinSock::*;

const DEFAULT_PORT: &HSTRING = w!("27015");

fn main() {
    let mut wsa_data = WSADATA::default();
    unsafe {
        let result = WSAStartup(0x0202, &mut wsa_data);
        if result != 0 {
            panic!("startup failed");
        }
    }

    if let Some(username) = std::env::args().nth(1) {
        user_loop(username);
    } else {
        server_loop();
    }
}

fn user_loop(username: String) {
    fn connect_user() -> SOCKET {
        let hints = ADDRINFOW {
            ai_family: AF_UNSPEC.0 as _,
            ai_socktype: SOCK_STREAM as _,
            ai_protocol: IPPROTO_TCP.0 as _,
            ..Default::default()
        };

        let mut ptr = std::ptr::null_mut();

        unsafe {
            let result = GetAddrInfoW(None, DEFAULT_PORT, Some(&hints), &mut ptr);
            if result != 0 {
                panic!("getaddrinfo failed");
            }
        }

        let mut connect_socket = INVALID_SOCKET;

        while !ptr.is_null() {
            let info = unsafe { *ptr };
            ptr = info.ai_next;
            connect_socket = unsafe { socket(info.ai_family, info.ai_socktype, info.ai_protocol) };

            if connect_socket == INVALID_SOCKET {
                unsafe {
                    panic!("socket is invalid: {:?}", WSAGetLastError());
                }
            }

            unsafe {
                let result = connect(connect_socket, info.ai_addr, info.ai_addrlen as _);
                if result == SOCKET_ERROR {
                    closesocket(connect_socket);
                    connect_socket = INVALID_SOCKET;
                    continue;
                }
            }
            break;
        }

        if connect_socket == INVALID_SOCKET {
            unsafe {
                panic!("socket is invalid: {:?}", WSAGetLastError());
            }
        }
        connect_socket
    }

    let connect_socket = connect_user();

    let regi_string = format!("regi:{}", username);

    unsafe {
        send(connect_socket, regi_string.as_bytes(), SEND_RECV_FLAGS(0));
    }

    std::thread::spawn(move || unsafe {
        let mut buffer = vec![0; 128];
        loop {
            if recv(connect_socket, &mut buffer, SEND_RECV_FLAGS(0)) > 0 {
                println!("{}", String::from_utf8_lossy(&buffer));
                buffer.clear();
            } else {
                std::thread::sleep(std::time::Duration::from_secs_f32(0.2));
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
                send(connect_socket, msg_string.as_bytes(), SEND_RECV_FLAGS(0));
            }
        }
        buf.clear();
    }
}

fn server_loop() {
    let hints = ADDRINFOW {
        ai_family: AF_INET.0 as _,
        ai_socktype: SOCK_STREAM as _,
        ai_protocol: IPPROTO_TCP.0 as _,
        ai_flags: AI_PASSIVE as _,
        ..Default::default()
    };

    let info = unsafe {
        let mut ptr = std::ptr::null_mut();
        let result = GetAddrInfoW(None, DEFAULT_PORT, Some(&hints), &mut ptr);
        if result != 0 {
            panic!("getaddrinfo failed");
        }
        *ptr
    };

    let listen_socket = unsafe { socket(info.ai_family, info.ai_socktype, info.ai_protocol) };

    if listen_socket == INVALID_SOCKET {
        unsafe {
            panic!("socket is invalid: {:?}", WSAGetLastError());
        }
    }

    unsafe {
        let result = bind(listen_socket, info.ai_addr, info.ai_addrlen as _);
        if result != 0 {
            panic!("bind failed");
        }
    }

    unsafe {
        let result = listen(listen_socket, SOMAXCONN as _);
        if result == SOCKET_ERROR {
            panic!("listen failed");
        }
    }

    use std::sync::{Arc, Mutex};

    let user_sockets = Arc::new(Mutex::new(BTreeMap::new()));

    unsafe {
        loop {
            let client_socket = accept(listen_socket, None, None);
            if client_socket == INVALID_SOCKET {
                continue;
            }

            let user_sockets = user_sockets.clone();

            std::thread::spawn(move || loop {
                let messages = Message::fetch(client_socket);

                if !messages.is_empty() {
                    for message in messages {
                        match message {
                            Message::Register { username, socket } => {
                                println!("{} registered", username);
                                if let Ok(mut sockets) = user_sockets.lock() {
                                    sockets.insert(username, socket);
                                }
                            }
                            Message::Text { from, to, text } => {
                                println!("text from {} to {:?}: {}", from, to, text);
                                let composed_message = format!("{}:{}", from, text);
                                for receiver in to {
                                    if let Some(socket) =
                                        user_sockets.lock().unwrap().get(&receiver)
                                    {
                                        send(
                                            socket.to_owned(),
                                            composed_message.as_bytes(),
                                            SEND_RECV_FLAGS(0),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_secs_f32(0.5));
            });
        }
    }
}

#[derive(Debug)]
enum Message {
    Register {
        username: String,
        socket: SOCKET,
    },
    Text {
        from: String,
        to: Vec<String>,
        text: String,
    },
}

impl Message {
    fn fetch(client_socket: SOCKET) -> Vec<Self> {
        let mut messages = vec![];

        unsafe {
            let mut buffer = vec![0; 60];
            if recv(client_socket, &mut buffer, SEND_RECV_FLAGS(0)) <= 0 {
                return messages;
            }

            let string = String::from_utf8(buffer).unwrap();

            match &string[..4] {
                "regi" => {
                    let mut splitted = string.split(':');
                    _ = splitted.next();
                    let username = splitted.next().unwrap().to_string();
                    let username = username.split('\0').next().unwrap().to_string();
                    messages.push(Self::Register {
                        username,
                        socket: client_socket,
                    });
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

        messages
    }
}
