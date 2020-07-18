use classicube_helpers::events::chat::{ChatReceivedEvent, ChatReceivedEventHandler};
use classicube_sys::{
    Entities, IGameComponent, MsgType_MSG_TYPE_NORMAL, WindowInfo, ENTITIES_SELF_ID,
};
use std::{
    ffi::CStr,
    fs::File,
    io,
    io::{BufRead, BufReader},
    mem::size_of,
    os::raw::{c_char, c_int},
    ptr,
};
use winapi::{
    shared::windef::HWND,
    um::winuser::{FlashWindowEx, GetActiveWindow, FLASHWINFO, FLASHW_TRAY},
};

fn flash_window() {
    unsafe {
        let cc_window = WindowInfo.Handle as HWND;
        let active_window = GetActiveWindow();
        if active_window != cc_window {
            let mut info = FLASHWINFO {
                cbSize: size_of::<FLASHWINFO>() as _,
                hwnd: cc_window,
                dwFlags: FLASHW_TRAY,
                uCount: 1,
                dwTimeout: 0,
            };
            FlashWindowEx(&mut info);
        }
    }
}

#[derive(Debug)]
enum Filter {
    Ignore(String),
    Contains(String),
}

fn parse_line(s: &str) -> Filter {
    if s.len() > 1 && s.starts_with('!') {
        let s = &s[1..];
        Filter::Ignore(s.to_lowercase())
    } else {
        Filter::Contains(s.to_lowercase())
    }
}

fn read_file(filters: &mut Vec<Filter>) -> io::Result<()> {
    match File::open("plugins/mentions.txt") {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                let line = line.trim();
                if line == "" {
                    continue;
                }

                filters.push(parse_line(line));
            }
        }

        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e);
            }
        }
    }

    Ok(())
}

extern "C" fn init() {
    println!(
        "init {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    thread_local!(
        static CHAT_RECEIVED: ChatReceivedEventHandler = {
            let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
            let c_str = unsafe { CStr::from_ptr(&me.NameRaw as *const c_char) };
            let my_name = c_str.to_string_lossy().to_string();

            let mut filters = Vec::new();

            if let Err(e) = read_file(&mut filters) {
                eprintln!("{:#?}", e);
            }

            filters.push(Filter::Ignore("is afk auto".to_string()));
            filters.push(Filter::Ignore("is no longer afk".to_string()));
            filters.push(Filter::Contains(my_name.to_lowercase()));

            println!("flashing mention filters: {:#?}", filters);

            let mut handler = ChatReceivedEventHandler::new();

            handler.on(
                move |ChatReceivedEvent {
                          message,
                          message_type,
                      }| {
                    if *message_type == MsgType_MSG_TYPE_NORMAL {
                        let message = message.to_lowercase();
                        for filter in &filters {
                            match filter {
                                Filter::Ignore(text) => {
                                    if message.contains(text) {
                                        break;
                                    }
                                }

                                Filter::Contains(text) => {
                                    if message.contains(text) {
                                        println!("mention {:#?}", filter);
                                        flash_window();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                },
            );

            handler
        };
    );

    CHAT_RECEIVED.with(|_| {});
}

#[no_mangle]
pub static Plugin_ApiVersion: c_int = 1;

#[no_mangle]
pub static mut Plugin_Component: IGameComponent = IGameComponent {
    // Called when the game is being loaded.
    Init: Some(init),
    // Called when the component is being freed. (e.g. due to game being closed)
    Free: None,
    // Called to reset the component's state. (e.g. reconnecting to server)
    Reset: None,
    // Called to update the component's state when the user begins loading a new map.
    OnNewMap: None,
    // Called to update the component's state when the user has finished loading a new map.
    OnNewMapLoaded: None,
    // Next component in linked list of components.
    next: ptr::null_mut(),
};
