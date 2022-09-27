#![windows_subsystem = "windows"]

use windows::core::*;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::{
    Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR},
    Gdi::*,
};
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use windows::Win32::System::Threading::CreateProcessW;
use windows::Win32::UI::WindowsAndMessaging::*;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();

    let text = args.join(" ");
    MainWindow::new(Some(text));
}

#[derive(Default)]
struct MainWindow {
    window: HWND,
    edit_field: HWND,
}

impl MainWindow {
    fn new(text: Option<String>) -> Self {
        let mut main_window = Self::default();

        unsafe {
            let class_name = w!("Hasher Window Class").into();

            let hinstance = GetModuleHandleW(None).expect("Can't get module handle");

            assert!(!hinstance.is_invalid());

            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(Self::wndproc),
                    hInstance: hinstance,
                    lpszClassName: class_name,
                    ..Default::default()
                }),
                0
            );

            main_window.window = CreateWindowExW(
                WS_EX_ACCEPTFILES | WS_EX_DLGMODALFRAME,
                class_name,
                w!("Gorynych"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                350,
                240,
                None,
                None,
                hinstance,
                Some(&mut main_window as *mut _ as _),
            );

            debug_assert!(main_window.window.0 != 0);

            let monofont = CreateFontW(
                24,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                DEFAULT_CHARSET.0,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                FF_DONTCARE,
                w!("Cascadia Code"),
            );

            main_window.edit_field = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("Edit"),
                w!(""),
                WS_VISIBLE | WS_CHILD | WS_VSCROLL | WINDOW_STYLE((ES_MULTILINE) as _),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                main_window.window,
                None,
                hinstance,
                None,
            );

            if let Some(text) = text {
                SetWindowTextW(main_window.edit_field, &HSTRING::from(text));
            }

            SendMessageW(
                main_window.edit_field,
                WM_SETFONT,
                WPARAM(monofont.0 as _),
                LPARAM(1),
            );

            _ = DwmSetWindowAttribute(
                main_window.window,
                DWMWA_CAPTION_COLOR,
                &[
                    0xEE, // Red
                    0xEE, // Green
                    0xEE, // Blue
                    0,    // Zero
                ],
            );

            ShowWindow(main_window.window, SW_SHOW);

            let mut message = MSG::default();

            while GetMessageW(&mut message, None, 0, 0).into() {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        main_window
    }

    extern "system" fn wndproc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            if message == WM_NCCREATE {
                let cs = lparam.0 as *const CREATESTRUCTA;
                let this = (*cs).lpCreateParams as *mut Self;
                (*this).window = window;

                SetWindowLongPtrW(window, GWLP_USERDATA, this as _);
            } else {
                let this = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Self;

                if !this.is_null() {
                    return (*this).message_handler(message, wparam, lparam);
                }
            }

            DefWindowProcW(window, message, wparam, lparam)
        }
    }

    fn message_handler(&self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match message {
                WM_GETMINMAXINFO => {
                    let min_size = &mut (*(lparam.0 as *mut MINMAXINFO)).ptMinTrackSize;
                    min_size.x = 200;
                    min_size.y = 120;
                }
                WM_SIZE => {
                    let width = (lparam.0 as u16) as i32;
                    let height = (lparam.0 >> 16) as i32;

                    MoveWindow(self.edit_field, 0, 0, width, height, true);
                    InvalidateRect(self.edit_field, None, None);
                }
                WM_DESTROY => {
                    for _ in 0..3 {
                        self.clone();
                    }

                    PostQuitMessage(0)
                }
                _ => return DefWindowProcW(self.window, message, wparam, lparam),
            };
            LRESULT(0)
        }
    }

    unsafe fn clone(&self) {
        let mut app_name: Vec<u16> = vec![0; MAX_PATH as _];
        GetModuleFileNameW(None, &mut app_name);

        let mut buff = vec![0; GetWindowTextLengthW(self.edit_field) as usize + 1];

        GetWindowTextW(self.edit_field, &mut buff);

        let mut command = Vec::from(w!("app ").as_wide());

        command.append(&mut buff);

        CreateProcessW(
            PCWSTR::from_raw(app_name.as_mut_ptr()),
            PWSTR::from_raw(command.as_mut_ptr()),
            None,
            None,
            false,
            Default::default(),
            Default::default(),
            PCWSTR::from_raw(0 as _),
            &Default::default() as _,
            &mut Default::default() as _,
        );
    }
}
