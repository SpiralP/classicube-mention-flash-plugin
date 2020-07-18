use classicube_helpers::events::chat::{ChatReceivedEvent, ChatReceivedEventHandler};
use classicube_sys::*;
use std::{
    ffi::CStr,
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

extern "C" fn init() {
    thread_local!(
        static CHAT_RECEIVED: ChatReceivedEventHandler = {
            let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
            let c_str = unsafe { CStr::from_ptr(&me.NameRaw as *const c_char) };
            let my_name = c_str.to_string_lossy().to_string();
            let my_name = my_name.to_lowercase();

            let mut handler = ChatReceivedEventHandler::new();

            handler.on(
                move |ChatReceivedEvent {
                          message,
                          message_type,
                      }| {
                    if *message_type == MsgType_MSG_TYPE_NORMAL
                        && message.to_lowercase().contains(&my_name)
                    {
                        flash_window();
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
