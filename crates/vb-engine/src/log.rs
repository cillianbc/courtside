//! The append-only event log. Nothing is ever removed except by undo,
//! which pops the tail. Edits to earlier events append too: a correction
//! replaces or removes its target, an insertion goes before an existing
//! event. The fold sees the result through [`EventLog::visible`].

use std::collections::HashMap;

use crate::event::{Body, EventId, IdGen, MatchEvent, NewEvent};

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

    /// Adds an event at the end of the match.
    pub fn append(&mut self, ids: &mut IdGen, new: NewEvent) -> EventId {
        self.push(ids, new, None, None, None)
    }

    pub(crate) fn push(
        &mut self,
        ids: &mut IdGen,
        new: NewEvent,
        supersedes: Option<EventId>,
        before: Option<EventId>,
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
            before,
            source_line,
        });
        id
    }

    /// Replaces `target` with `new` at the same place in the match. The
    /// target stays in the log, hidden from the fold. A correction can
    /// itself be corrected; an event can be corrected only once.
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
        Ok(self.push(ids, new, Some(target), None, None))
    }

    /// Hides `target` from the match: a correction with no replacement.
    pub fn remove(
        &mut self,
        ids: &mut IdGen,
        target: EventId,
        producer: crate::event::Producer,
    ) -> Result<EventId, LogError> {
        let new = NewEvent {
            body: Body::Removed,
            video_ms: None,
            clock: None,
            producer,
        };
        self.correct(ids, target, new)
    }

    /// Adds `new` to the match just before `target` (or before what
    /// `target` corrects, if it is a correction).
    pub fn insert_before(
        &mut self,
        ids: &mut IdGen,
        target: EventId,
        new: NewEvent,
    ) -> Result<EventId, LogError> {
        let anchor = self.root(target).ok_or(LogError::NotFound(target))?;
        Ok(self.push(ids, new, None, Some(anchor), None))
    }

    /// Removes the last event (or correction or insertion), undoing it.
    pub fn undo(&mut self) -> Option<MatchEvent> {
        self.events.pop()
    }

    /// The original or inserted event a correction chain starts from.
    fn root(&self, id: EventId) -> Option<EventId> {
        let mut cur = self.get(id)?;
        while let Some(t) = cur.supersedes {
            cur = self.get(t)?;
        }
        Some(cur.id)
    }

    /// The events the fold sees, in match order: each original or
    /// inserted event (insertions just before their anchor, in the order
    /// they were made), or its latest correction in its place. Removed
    /// events are left out.
    pub fn visible(&self) -> Vec<&MatchEvent> {
        let mut replaced_by: HashMap<EventId, &MatchEvent> = HashMap::new();
        let mut inserted: HashMap<EventId, Vec<&MatchEvent>> = HashMap::new();
        for e in &self.events {
            if let Some(t) = e.supersedes {
                replaced_by.insert(t, e);
            } else if let Some(anchor) = e.before {
                inserted.entry(anchor).or_default().push(e);
            }
        }
        let mut out = Vec::with_capacity(self.events.len());
        // Depth-first without recursion: an event's insertions, then itself.
        let mut stack: Vec<(&MatchEvent, bool)> = self
            .events
            .iter()
            .rev()
            .filter(|e| e.supersedes.is_none() && e.before.is_none())
            .map(|e| (e, false))
            .collect();
        while let Some((e, expanded)) = stack.pop() {
            if !expanded {
                stack.push((e, true));
                if let Some(ins) = inserted.get(&e.id) {
                    stack.extend(ins.iter().rev().map(|i| (*i, false)));
                }
                continue;
            }
            let mut cur = e;
            while let Some(next) = replaced_by.get(&cur.id) {
                cur = next;
            }
            if cur.body != Body::Removed {
                out.push(cur);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Producer;
    use proptest::prelude::*;
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

    #[test]
    fn remove_and_insert() {
        let mut ids = IdGen::new(0);
        let mut log = EventLog::new();
        let s = log.append(&mut ids, new("*12SH-"));
        let a = log.append(&mut ids, new("a07AH#"));
        // The scout missed the reception.
        log.insert_before(&mut ids, a, new("a07RH#"))
            .expect("insert");
        assert_eq!(codes(&log), ["*12SH-", "a07RH#", "a07AH#"]);
        // Inserting before a correction goes before what it corrects.
        let c = log.correct(&mut ids, a, new("a10AH#")).expect("correct");
        log.insert_before(&mut ids, c, new("a07EH#"))
            .expect("insert");
        assert_eq!(codes(&log), ["*12SH-", "a07RH#", "a07EH#", "a10AH#"]);
        log.remove(&mut ids, s, Producer::Keyboard).expect("remove");
        assert_eq!(codes(&log), ["a07RH#", "a07EH#", "a10AH#"]);
        log.undo();
        assert_eq!(codes(&log), ["*12SH-", "a07RH#", "a07EH#", "a10AH#"]);
    }

    #[derive(Debug, Clone)]
    enum Op {
        Append(u8),
        Correct(usize, u8),
        Remove(usize),
        Insert(usize, u8),
        Undo,
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            3 => any::<u8>().prop_map(Op::Append),
            2 => (any::<usize>(), any::<u8>()).prop_map(|(i, c)| Op::Correct(i, c)),
            1 => any::<usize>().prop_map(Op::Remove),
            2 => (any::<usize>(), any::<u8>()).prop_map(|(i, c)| Op::Insert(i, c)),
            1 => Just(Op::Undo),
        ]
    }

    proptest! {
        /// The log's visible events always equal a plain list edited the
        /// same way, and undo restores the list before each edit.
        #[test]
        fn visible_matches_a_simple_list(ops in prop::collection::vec(op(), 0..60)) {
            let code = |c: u8| format!("*{:02}SH-", c % 100);
            let mut ids = IdGen::new(0);
            let mut log = EventLog::new();
            let mut list: Vec<String> = Vec::new();
            let mut history: Vec<Vec<String>> = Vec::new();
            for op in ops {
                let visible: Vec<EventId> = log.visible().iter().map(|e| e.id).collect();
                let before = list.clone();
                let pick = |i: usize| (!visible.is_empty()).then(|| i % visible.len());
                let changed = match op {
                    Op::Append(c) => {
                        log.append(&mut ids, new(&code(c)));
                        list.push(code(c));
                        true
                    }
                    Op::Correct(i, c) => pick(i).is_some_and(|i| {
                        log.correct(&mut ids, visible[i], new(&code(c))).expect("correct");
                        list[i] = code(c);
                        true
                    }),
                    Op::Remove(i) => pick(i).is_some_and(|i| {
                        log.remove(&mut ids, visible[i], Producer::Keyboard).expect("remove");
                        list.remove(i);
                        true
                    }),
                    Op::Insert(i, c) => pick(i).is_some_and(|i| {
                        log.insert_before(&mut ids, visible[i], new(&code(c))).expect("insert");
                        list.insert(i, code(c));
                        true
                    }),
                    Op::Undo => {
                        if log.undo().is_some() {
                            list = history.pop().expect("history");
                        }
                        false
                    }
                };
                if changed {
                    history.push(before);
                }
                prop_assert_eq!(codes(&log), list.clone());
            }
        }
    }
}
