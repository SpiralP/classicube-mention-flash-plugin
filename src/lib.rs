mod error;

use crate::error::*;
use classicube_helpers::{
    events::chat::{ChatReceivedEvent, ChatReceivedEventHandler},
    tab_list::remove_color,
};
use classicube_sys::{
    Chat_Add, Entities, IGameComponent, MsgType_MSG_TYPE_NORMAL, OwnedString, WindowInfo,
    ENTITIES_SELF_ID,
};
use regex::Regex;
use std::{
    ffi::CStr,
    fmt::Debug,
    fs::File,
    io,
    io::{BufRead, BufReader, Write},
    os::raw::{c_char, c_int},
    ptr,
};

const MENTIONS_PATH: &str = "plugins/mentions.txt";

#[cfg(windows)]
fn flash_window() -> Result<()> {
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
fn flash_window() -> Result<()> {
    use std::{ffi::CString, mem};
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

        let display = XOpenDisplay(ptr::null_mut());
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

#[test]
fn test_flash_window() {
    flash_window().unwrap();
}

#[derive(Debug)]
enum Matcher {
    Contains(String),
    StartsWith(String),
    EndsWith(String),
    Regex(Regex),
}
impl Matcher {
    fn matches(&self, text: &str) -> bool {
        let text = remove_color(text);
        match self {
            Matcher::Contains(part) => text.to_lowercase().contains(&part.to_lowercase()),
            Matcher::StartsWith(part) => text.to_lowercase().starts_with(&part.to_lowercase()),
            Matcher::EndsWith(part) => text.to_lowercase().ends_with(&part.to_lowercase()),
            Matcher::Regex(regex) => regex.is_match(&text),
        }
    }
}

fn parse_line(s: &str) -> Result<Matcher> {
    if s.starts_with("contains:") {
        let s = &s[9..];
        Ok(Matcher::Contains(s.to_lowercase()))
    } else if s.starts_with("starts with:") {
        let s = &s[12..];
        Ok(Matcher::StartsWith(s.to_lowercase()))
    } else if s.starts_with("ends with:") {
        let s = &s[10..];
        Ok(Matcher::EndsWith(s.to_lowercase()))
    } else if s.starts_with("regex:") {
        let s = &s[6..];
        Ok(Matcher::Regex(Regex::new(s)?))
    } else {
        bail!("couldn't create matcher for {:?}", s);
    }
}

fn read_file(matches: &mut Vec<Matcher>, ignores: &mut Vec<Matcher>) -> Result<()> {
    match File::open(MENTIONS_PATH) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line == "" {
                    continue;
                }

                if line.starts_with("not ") {
                    let line = &line[4..];
                    ignores.push(parse_line(&line)?);
                } else {
                    matches.push(parse_line(&line)?);
                }
            }
        }

        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e.into());
            } else {
                {
                    let mut f = File::create(MENTIONS_PATH)?;

                    let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
                    let c_str = unsafe { CStr::from_ptr(&me.NameRaw as *const c_char) };
                    let my_name = c_str.to_string_lossy().to_string();
                    writeln!(f, "contains:{}", my_name)?;

                    writeln!(f, "starts with:[>] ")?;
                    writeln!(f, "not contains:went to")?;
                    writeln!(f, "not contains:is afk auto")?;
                    writeln!(f, "not contains:is no longer afk")?;
                }

                return read_file(matches, ignores);
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
            let mut matchers = Vec::new();
            let mut ignores = Vec::new();

            if let Err(e) = read_file(&mut matchers, &mut ignores) {
                eprintln!("{:#?}", e);
                print(format!("&cmentions.txt: &f{}", e));
            }

            println!("flashing mention matchers: {:#?}", matchers);
            println!("flashing mention ignores: {:#?}", ignores);

            let mut handler = ChatReceivedEventHandler::new();

            handler.on(
                move |ChatReceivedEvent {
                          message,
                          message_type,
                      }| {
                    if *message_type != MsgType_MSG_TYPE_NORMAL {
                        return;
                    }

                    for ignore in &ignores {
                        if ignore.matches(message) {
                            return;
                        }
                    }

                    for matcher in &matchers {
                        if matcher.matches(message) {
                            println!("mention {:#?}", matcher);
                            flash_window().unwrap();
                            break;
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

pub fn print<S: Into<String>>(s: S) {
    let mut s = s.into();

    if s.len() > 255 {
        s.truncate(255);
    }

    let owned_string = OwnedString::new(s);

    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
}
