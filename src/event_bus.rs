pub struct EventBus<E> {
    events: Vec<E>,
}

impl<E> EventBus<E> {
    pub const fn new() -> Self {
        Self { events: vec![] }
    }

    pub fn send(&mut self, event: E) {
        self.events.push(event);
    }

    pub fn read(&mut self) -> Option<E> {
        let length = self.events.len();
        if length > 0 {
            return Some(self.events.remove(length - 1));
        }
        None
    }
}
