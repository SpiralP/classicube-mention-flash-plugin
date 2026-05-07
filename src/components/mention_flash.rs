use std::{
    cell::RefCell,
    ffi::CStr,
    fmt::Debug,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    os::raw::c_char,
};

use anyhow::{Result, bail};
use classicube_helpers::{
    chat,
    events::chat::{ChatReceivedEvent, ChatReceivedEventHandler},
    tab_list::remove_color,
};
use classicube_sys::{ENTITIES_SELF_ID, Entities, MsgType_MSG_TYPE_NORMAL};
use regex::Regex;
use tracing::debug;

use crate::{component::Component, flash};

const MENTIONS_PATH: &str = "plugins/mentions.txt";

thread_local!(
    static CHAT_RECEIVED: RefCell<Option<ChatReceivedEventHandler>> = const { RefCell::new(None) };
);

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
    if let Some(s) = s.strip_prefix("contains:") {
        Ok(Matcher::Contains(s.to_lowercase()))
    } else if let Some(s) = s.strip_prefix("starts with:") {
        Ok(Matcher::StartsWith(s.to_lowercase()))
    } else if let Some(s) = s.strip_prefix("ends with:") {
        Ok(Matcher::EndsWith(s.to_lowercase()))
    } else if let Some(s) = s.strip_prefix("regex:") {
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
                if line.is_empty() {
                    continue;
                }

                if let Some(line) = line.strip_prefix("not ") {
                    ignores.push(parse_line(line)?);
                } else {
                    matches.push(parse_line(&line)?);
                }
            }
            Ok(())
        }

        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut f = File::create(MENTIONS_PATH)?;

            let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
            let c_str = unsafe { CStr::from_ptr(&raw const me.NameRaw as *const c_char) };
            let my_name = c_str.to_string_lossy().to_string();
            writeln!(f, "contains:{my_name}")?;

            writeln!(f, "starts with:[>] ")?;
            writeln!(f, "not contains:went to")?;
            writeln!(f, "not contains:is afk auto")?;
            writeln!(f, "not contains:is no longer afk")?;

            drop(f);
            read_file(matches, ignores)
        }

        Err(e) => Err(e.into()),
    }
}

pub struct MentionFlash;

impl Component for MentionFlash {
    fn name(&self) -> &'static str {
        "MentionFlash"
    }

    fn init(&mut self) {
        let mut matchers = Vec::new();
        let mut ignores = Vec::new();

        if let Err(e) = read_file(&mut matchers, &mut ignores) {
            eprintln!("{e:#?}");
            chat::print(format!("&cmentions.txt: &f{e}"));
        }

        debug!("flashing mention matchers: {matchers:#?}");
        debug!("flashing mention ignores: {ignores:#?}");

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
                        debug!("mention {matcher:#?}");
                        flash::flash_window().unwrap();
                        break;
                    }
                }
            },
        );

        CHAT_RECEIVED.with(|cell| *cell.borrow_mut() = Some(handler));
    }

    fn free(&mut self) {
        CHAT_RECEIVED.with(|cell| {
            cell.borrow_mut().take();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_is_case_insensitive() {
        let m = Matcher::Contains("spiralp".into());
        assert!(m.matches("hello SpiralP"));
        assert!(m.matches("HELLO SPIRALP!"));
        assert!(!m.matches("nobody here"));
    }

    #[test]
    fn starts_with_matches_after_color_strip() {
        let m = Matcher::StartsWith("[>] ".into());
        assert!(m.matches("&7[>] &fhello"));
        assert!(!m.matches("hello [>] world"));
    }

    #[test]
    fn ends_with_matches_after_color_strip() {
        let m = Matcher::EndsWith("!".into());
        assert!(m.matches("hi&f!"));
        assert!(!m.matches("hi! "));
    }

    #[test]
    fn regex_matcher() {
        let m = Matcher::Regex(Regex::new(r"^\[server\]").unwrap());
        assert!(m.matches("[server] joined the game"));
        assert!(!m.matches("not a [server] line"));
    }

    #[test]
    fn parse_line_recognizes_each_prefix() {
        assert!(matches!(parse_line("contains:foo").unwrap(), Matcher::Contains(s) if s == "foo"));
        assert!(
            matches!(parse_line("starts with:foo").unwrap(), Matcher::StartsWith(s) if s == "foo")
        );
        assert!(
            matches!(parse_line("ends with:foo").unwrap(), Matcher::EndsWith(s) if s == "foo")
        );
        assert!(matches!(
            parse_line("regex:^foo$").unwrap(),
            Matcher::Regex(_)
        ));
    }

    #[test]
    fn parse_line_lowercases_string_matchers() {
        // text.to_lowercase() in matches() means the parsed pattern must
        // also be lowercase, otherwise matches against mixed-case input
        // would silently miss.
        let Matcher::Contains(s) = parse_line("contains:SpiralP").unwrap() else {
            panic!()
        };
        assert_eq!(s, "spiralp");
    }

    #[test]
    fn parse_line_rejects_unknown_prefix() {
        assert!(parse_line("unknown:foo").is_err());
        assert!(parse_line("").is_err());
    }

    #[test]
    fn parse_line_propagates_regex_error() {
        assert!(parse_line("regex:[unclosed").is_err());
    }
}

