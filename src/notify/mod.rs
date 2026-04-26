pub mod notify_send;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Urgency {
    Normal,
    Critical,
}

pub trait Notifier: Send + Sync {
    fn translation_failed(&self, title: &str, message: &str, urgency: Urgency);
}
