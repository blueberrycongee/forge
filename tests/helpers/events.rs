use std::sync::{Arc, Mutex};

use forge::runtime::error::GraphResult;
use forge::runtime::event::{Event, EventSink};

/// Capture runtime events for test assertions.
#[derive(Clone, Default)]
pub struct EventCollector {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventCollector {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn sink(&self) -> Arc<dyn EventSink> {
        Arc::new(CollectorSink {
            events: Arc::clone(&self.events),
        })
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }
}

struct CollectorSink {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventSink for CollectorSink {
    fn emit(&self, event: Event) -> GraphResult<()> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}
