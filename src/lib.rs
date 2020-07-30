mod error;
mod flash;

use crate::error::*;
use classicube_helpers::{
    events::chat::{ChatReceivedEvent, ChatReceivedEventHandler},
    tab_list::remove_color,
};
use classicube_sys::{
    Chat_Add, Entities, IGameComponent, MsgType_MSG_TYPE_NORMAL, OwnedString, ENTITIES_SELF_ID,
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
                            flash::flash_window().unwrap();
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
