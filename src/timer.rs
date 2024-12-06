use std::{cell::Cell, time::{Duration, Instant}};

pub struct Timer<'me> {
    start: Instant,
    field: &'me Cell<Duration>,
}


impl<'me> Timer<'me> {
    pub fn new(to: &'me Cell<Duration>) -> Self {
        Self { start: Instant::now(), field: to }
    }

}


impl<'me> Drop for Timer<'me> {
    fn drop(&mut self) {
        self.field.set(self.start.elapsed());
    }
}
