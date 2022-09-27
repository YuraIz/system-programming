use windows::core::*;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::{
    Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR},
    Gdi::*,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use windows::Win32::UI::{Controls::Dialogs::*, Shell::*, WindowsAndMessaging::*};

fn main() {
    HasherWindow::new();
}

#[derive(Default)]
struct HasherWindow {
    window: HWND,
    drop_text: HWND,
    checksums_text: HWND,
    button: HWND,

    text_shown: std::cell::Cell<bool>,

    background: HBRUSH,
    foreground: COLORREF,
}

impl HasherWindow {
    fn new() -> Self {
        let mut hasher_window = Self::default();

        unsafe {
            hasher_window.background = CreateSolidBrush(COLORREF(0x202020));
            hasher_window.foreground = COLORREF(0xFFFFFF);

            let class_name = w!("Hasher Window Class").into();

            let hinstance = GetModuleHandleW(None).expect("Can't get module handle");
            debug_assert!(!hinstance.is_invalid());

            let wc = WNDCLASSW {
                lpfnWndProc: Some(Self::wndproc),
                hInstance: hinstance,
                lpszClassName: class_name,
                ..Default::default()
            };

            let res = RegisterClassW(&wc);

            assert!(res != 0);

            let window = CreateWindowExW(
                WS_EX_ACCEPTFILES | WS_EX_DLGMODALFRAME,
                class_name,
                w!("Hasher"),
                WS_OVERLAPPEDWINDOW,
                // Size and position
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                350,
                240,
                None,
                None,
                hinstance,
                Some(&mut hasher_window as *mut _ as _),
            );

            debug_assert!(window.0 != 0);

            let big_font = CreateFontW(
                30,
                0,
                0,
                0,
                500,
                0,
                0,
                0,
                DEFAULT_CHARSET.0,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                FF_DONTCARE,
                w!("SEGUIVAR"),
            );

            let small_font = CreateFontW(
                20,
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
                w!("SEGUIVAR"),
            );

            // dbg!(SendMessageW(window, WM_GETFONT, WPARAM(0), LPARAM(0)));

            // DragAcceptFiles(window, true);

            hasher_window.button = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Open"),
                WS_TABSTOP | WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                100,
                30,
                window,
                None,
                HINSTANCE(GetWindowLongPtrW(window, GWLP_HINSTANCE)),
                None,
            );

            SendMessageW(
                hasher_window.button,
                WM_SETFONT,
                WPARAM(small_font.0 as _),
                LPARAM(1),
            );

            hasher_window.drop_text = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Drop files here"),
                WS_VISIBLE | WS_CHILD,
                10,
                10,
                100,
                30,
                window,
                None,
                HINSTANCE(GetWindowLongPtrW(window, GWLP_HINSTANCE)),
                None,
            );

            SendMessageW(
                hasher_window.drop_text,
                WM_SETFONT,
                WPARAM(big_font.0 as _),
                LPARAM(1),
            );

            hasher_window.checksums_text = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("Edit"),
                w!("CheckSums"),
                WS_CHILD | WS_VSCROLL | WINDOW_STYLE((ES_MULTILINE | ES_READONLY) as _),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                200,
                100,
                window,
                None,
                HINSTANCE(GetWindowLongPtrW(window, GWLP_HINSTANCE)),
                None,
            );

            SendMessageW(
                hasher_window.checksums_text,
                WM_SETFONT,
                WPARAM(small_font.0 as _),
                LPARAM(1),
            );

            let color = &[
                0x20, // Red
                0x20, // Green
                0x20, // Blue
                0,    // Zero
            ];

            _ = DwmSetWindowAttribute(window, DWMWA_CAPTION_COLOR, color);

            ShowWindow(window, SW_SHOW);

            let mut message = MSG::default();

            while GetMessageW(&mut message, None, 0, 0).into() {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        hasher_window
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
                    min_size.x = 300;
                    min_size.y = 180;
                }
                WM_NCHITTEST => {
                    return match DefWindowProcW(self.window, message, wparam, lparam) {
                        LRESULT(1) => LRESULT(2), // All window is draggable area
                        other => other,           // Borders, Buttons, etc
                    };
                }
                WM_SIZE => {
                    let width = (lparam.0 as u16) as i32;
                    let height = (lparam.0 >> 16) as i32;

                    MoveWindow(
                        self.drop_text,
                        width / 2 - 80,
                        height / 2 - 40,
                        200,
                        30,
                        true,
                    );

                    if self.text_shown.get() {
                        MoveWindow(self.button, 8, 0, 60, 30, true);
                    } else {
                        MoveWindow(self.button, width / 2 - 30, height / 2 + 20, 60, 30, true);
                    }

                    MoveWindow(self.checksums_text, 8, 30, width - 8, height, true);
                    InvalidateRect(self.checksums_text, None, None);
                }
                WM_PAINT => {
                    let mut paint = PAINTSTRUCT::default();

                    let hdc = BeginPaint(self.window, &mut paint);

                    FillRect(hdc, &paint.rcPaint, self.background);

                    EndPaint(self.window, &paint);
                }
                WM_CTLCOLORSTATIC => {
                    let hdc = HDC(wparam.0 as isize);

                    SetTextColor(hdc, self.foreground);
                    SetBkMode(hdc, TRANSPARENT);

                    return LRESULT(self.background.0);
                }
                WM_CTLCOLORBTN => {
                    return LRESULT(self.background.0);
                }
                WM_DESTROY => PostQuitMessage(0),

                WM_DROPFILES => self.drop_files_handler(wparam),

                WM_COMMAND => {
                    if lparam.0 == self.button.0 {
                        self.open_file();
                    } else {
                        return DefWindowProcW(self.window, message, wparam, lparam);
                    }
                }

                _ => return DefWindowProcW(self.window, message, wparam, lparam),
            };
            LRESULT(0)
        }
    }

    unsafe fn open_file(&self) {
        let mut buff: Vec<u16> = vec![0; 260];

        let mut openfilename = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as _,
            hwndOwner: self.window,
            lpstrFile: PWSTR::from_raw(buff.as_mut_ptr()),
            nMaxFile: buff.len() as _,

            ..Default::default()
        };

        GetOpenFileNameW(&mut openfilename as *mut _);

        let filename = &{
            String::from_utf16(&buff)
                .unwrap_or_default()
                .split('\0')
                .next()
                .unwrap_or_default()
                .to_owned()
        };

        let name = {
            let splitted = filename.split('\\');
            splitted.last().unwrap_or_default()
        };

        let mut result_string = String::new();

        result_string.push_str(&format!("{}:\r\n\r\n", name));

        for algorhitm in ["MD2", "MD4", "MD5", "SHA1", "SHA256", "SHA384", "SHA512"] {
            if let Some(hash) = Self::hash_for_filename(filename, algorhitm) {
                result_string.push_str(&format!("{}: {}\r\n", algorhitm, hash));
            }
        }
        result_string.push_str("\r\n");

        self.set_text(&result_string);
    }

    unsafe fn drop_files_handler(&self, wparam: WPARAM) {
        let file_count = DragQueryFileW(HDROP(wparam.0 as isize), 0xFFFFFFFF, None);

        let mut filenames = vec![];

        let mut buffer = vec![];

        for file_index in 0..file_count {
            let buf_size = DragQueryFileW(HDROP(wparam.0 as isize), file_index, None) as usize;

            buffer.resize(buf_size + 1, 0);

            DragQueryFileW(
                HDROP(wparam.0 as isize),
                file_index,
                Some(buffer.as_mut_slice()),
            );

            let filename = String::from_utf16(&buffer[..buf_size]).unwrap();
            filenames.push(filename);
        }

        let mut result_string = String::new();

        filenames.sort_unstable();

        for filename in &filenames {
            let name = {
                let splitted = filename.split('\\');
                splitted.last().unwrap_or_default()
            };

            result_string.push_str(&format!("{}:\r\n\r\n", name));

            for algorhitm in ["MD2", "MD4", "MD5", "SHA1", "SHA256", "SHA384", "SHA512"] {
                if let Some(hash) = Self::hash_for_filename(filename, algorhitm) {
                    result_string.push_str(&format!("{}: {}\r\n", algorhitm, hash));
                }
            }
            result_string.push_str("\r\n");
        }

        // ShowWindow(self.drop_text, SW_HIDE);
        // ShowWindow(self.checksums_text, SW_NORMAL);
        // SetWindowTextW(self.checksums_text, &HSTRING::from(result_string));

        self.set_text(&result_string);

        // ShowScrollBar(self.checksums_text, SB_VERT, false);
    }

    unsafe fn set_text(&self, result_string: &str) {
        if result_string != ":\r\n\r\n\r\n" {
            self.text_shown.set(true);

            ShowWindow(self.drop_text, SW_HIDE);
            ShowWindow(self.checksums_text, SW_NORMAL);
            SetWindowTextW(self.checksums_text, &HSTRING::from(result_string));
            MoveWindow(self.button, 8, 0, 60, 30, true);
        } else {
            self.text_shown.set(false);
            ShowWindow(self.drop_text, SW_NORMAL);
            ShowWindow(self.checksums_text, SW_HIDE);
            InvalidateRect(self.button, None, None);

            let mut rect = RECT::default();

            GetWindowRect(self.window, &mut rect as *mut _);

            let width = rect.right - rect.left - 16;
            let height = rect.bottom - rect.top - 39;
            MoveWindow(self.button, width / 2 - 30, height / 2 + 20, 60, 30, true);
        }
    }

    fn hash_for_filename(filename: &str, algorhitm: &str) -> Option<String> {
        let output = std::process::Command::new("certutil")
            .args(&["-hashfile", filename, algorhitm])
            .output()
            .ok()?;

        let output = String::from_utf8(output.stdout).ok()?;

        let hash = output.split("\r\n").nth(1)?;
        if hash == "CertUtil: The system cannot find the file specified." {
            None
        } else {
            Some(hash.to_string())
        }
    }
}
