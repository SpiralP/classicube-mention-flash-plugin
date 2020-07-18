use classicube_helpers::events::chat::{ChatReceivedEvent, ChatReceivedEventHandler};
use classicube_sys::{
    Entities, IGameComponent, MsgType_MSG_TYPE_NORMAL, WindowInfo, ENTITIES_SELF_ID,
};
use std::{
    collections::HashSet,
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

fn read_file(mentions: &mut HashSet<String>) -> io::Result<()> {
    match File::open("plugins/mentions.txt") {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                let line = line.trim();
                if line == "" {
                    continue;
                }

                mentions.insert(line.to_lowercase().to_string());
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

            let mut mentions = HashSet::new();
            mentions.insert(my_name.to_lowercase());

            if let Err(e) = read_file(&mut mentions) {
                eprintln!("{:#?}", e);
            }

            println!("flashing mentions: {:#?}", mentions);

            let mut handler = ChatReceivedEventHandler::new();

            handler.on(
                move |ChatReceivedEvent {
                          message,
                          message_type,
                      }| {
                    if *message_type == MsgType_MSG_TYPE_NORMAL {
                        let message = message.to_lowercase();
                        for mention in &mentions {
                            if message.contains(mention) {
                                flash_window();
                                break;
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
