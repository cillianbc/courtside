//! The append-only event log. Nothing is ever removed except by undo,
//! which pops the tail; corrections append and hide what they replace.

use std::collections::HashMap;

use crate::event::{EventId, IdGen, MatchEvent, NewEvent};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LogError {
    #[error("no event {0} in the log")]
    NotFound(EventId),
    #[error("event {0} is already superseded by {1}")]
    AlreadySuperseded(EventId, EventId),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventLog {
    events: Vec<MatchEvent>,
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every event, including superseded ones and corrections, in log order.
    pub fn all(&self) -> &[MatchEvent] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn get(&self, id: EventId) -> Option<&MatchEvent> {
        self.events.iter().find(|e| e.id == id)
    }

    pub fn append(&mut self, ids: &mut IdGen, new: NewEvent) -> EventId {
        self.push(ids, new, None, None)
    }

    pub(crate) fn push(
        &mut self,
        ids: &mut IdGen,
        new: NewEvent,
        supersedes: Option<EventId>,
        source_line: Option<usize>,
    ) -> EventId {
        let id = ids.next_id();
        let seq = u32::try_from(self.events.len()).unwrap_or(u32::MAX);
        self.events.push(MatchEvent {
            id,
            seq,
            body: new.body,
            video_ms: new.video_ms,
            clock: new.clock,
            producer: new.producer,
            supersedes,
            source_line,
        });
        id
    }

    /// Replaces `target` with `new` at the same place in the match. The
    /// target stays in the log, hidden from the fold.
    pub fn correct(
        &mut self,
        ids: &mut IdGen,
        target: EventId,
        new: NewEvent,
    ) -> Result<EventId, LogError> {
        if self.get(target).is_none() {
            return Err(LogError::NotFound(target));
        }
        if let Some(by) = self.events.iter().find(|e| e.supersedes == Some(target)) {
            return Err(LogError::AlreadySuperseded(target, by.id));
        }
        Ok(self.push(ids, new, Some(target), None))
    }

    /// Removes the last event (or correction), un-hiding what it replaced.
    pub fn undo(&mut self) -> Option<MatchEvent> {
        self.events.pop()
    }

    /// The events the fold sees, in match order: each original event, or
    /// its latest correction in its place.
    pub fn visible(&self) -> Vec<&MatchEvent> {
        let replaced_by: HashMap<EventId, &MatchEvent> = self
            .events
            .iter()
            .filter_map(|e| e.supersedes.map(|t| (t, e)))
            .collect();
        self.events
            .iter()
            .filter(|e| e.supersedes.is_none())
            .map(|e| {
                let mut cur = e;
                while let Some(next) = replaced_by.get(&cur.id) {
                    cur = next;
                }
                cur
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Producer;
    use vb_grammar::Code;

    fn new(s: &str) -> NewEvent {
        NewEvent::code(Code::parse(s), Producer::Keyboard)
    }

    fn codes(log: &EventLog) -> Vec<String> {
        log.visible()
            .iter()
            .filter_map(|e| e.code().map(ToString::to_string))
            .collect()
    }

    #[test]
    fn correction_takes_the_place_of_its_target() {
        let mut ids = IdGen::new(0);
        let mut log = EventLog::new();
        log.append(&mut ids, new("*12SH-"));
        let r = log.append(&mut ids, new("a07RH#"));
        log.append(&mut ids, new("a07AH#"));
        let c = log.correct(&mut ids, r, new("a08RH#")).expect("correct");
        assert_eq!(codes(&log), ["*12SH-", "a08RH#", "a07AH#"]);
        // A correction can itself be corrected.
        log.correct(&mut ids, c, new("a09RH+")).expect("correct");
        assert_eq!(codes(&log), ["*12SH-", "a09RH+", "a07AH#"]);
        assert_eq!(log.len(), 5);
        assert_eq!(
            log.correct(&mut ids, r, new("a01RH#")),
            Err(LogError::AlreadySuperseded(r, c))
        );
        log.undo();
        assert_eq!(codes(&log), ["*12SH-", "a08RH#", "a07AH#"]);
    }
}
