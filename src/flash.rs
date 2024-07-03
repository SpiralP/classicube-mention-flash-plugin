use anyhow::Result;
use classicube_sys::WindowInfo;

#[cfg(windows)]
pub fn flash_window() -> Result<()> {
    use std::mem::size_of;
    use winapi::{
        shared::windef::HWND,
        um::winuser::{FlashWindowEx, GetActiveWindow, FLASHWINFO, FLASHW_TRAY},
    };

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

    Ok(())
}

#[cfg(unix)]
pub fn flash_window() -> Result<()> {
    use anyhow::bail;
    use std::{ffi::CString, mem, ptr};
    use x11::xlib::{
        Atom, ClientMessage, Display, False, SubstructureNotifyMask, SubstructureRedirectMask,
        True, Window, XCloseDisplay, XDefaultRootWindow, XEvent, XInternAtom, XOpenDisplay,
        XSendEvent,
    };

    const _NET_WM_STATE_REMOVE: u64 = 0; // remove/unset property
    const _NET_WM_STATE_ADD: u64 = 1; // add/set property
    const _NET_WM_STATE_TOGGLE: u64 = 2; // toggle property

    #[link(name = "X11")]
    extern "C" {}

    #[allow(non_snake_case)]
    unsafe {
        unsafe fn atom<T: Into<Vec<u8>>>(display: *mut Display, name: T) -> Result<Atom> {
            let name = CString::new(name)?;
            let atom = XInternAtom(display, name.as_ptr(), True);
            if atom == 0 {
                bail!("XInternAtom {:?}", name);
            } else {
                Ok(atom)
            }
        }

        let window = WindowInfo.Handle as Window;

        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            bail!("XOpenDisplay");
        }

        let _NET_WM_STATE = atom(display, "_NET_WM_STATE")?;
        let _NET_WM_STATE_DEMANDS_ATTENTION = atom(display, "_NET_WM_STATE_DEMANDS_ATTENTION")?;

        let mut event: XEvent = mem::zeroed();
        let mask = SubstructureRedirectMask | SubstructureNotifyMask;

        event.client_message.type_ = ClientMessage;
        event.client_message.serial = 0;
        event.client_message.send_event = True;
        event.client_message.message_type = _NET_WM_STATE;
        event.client_message.window = window;
        event.client_message.format = 32;
        event
            .client_message
            .data
            .set_long(0, _NET_WM_STATE_ADD as _);
        event
            .client_message
            .data
            .set_long(1, _NET_WM_STATE_DEMANDS_ATTENTION as _);

        if XSendEvent(
            display,
            XDefaultRootWindow(display),
            False,
            mask,
            &mut event,
        ) == 0
        {
            bail!("XSendEvent");
        }

        XCloseDisplay(display);
    }

    Ok(())
}
