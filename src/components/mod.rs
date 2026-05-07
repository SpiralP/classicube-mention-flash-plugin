pub mod logger;
pub mod mention_flash;

use crate::{
    component::Component,
    components::{logger::Logger, mention_flash::MentionFlash},
};

pub fn init_components() -> Vec<Box<dyn Component>> {
    vec![Box::new(Logger), Box::new(MentionFlash)]
}
