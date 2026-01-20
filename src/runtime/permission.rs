//! Permission evaluation primitives for tool/operation gating.

use serde::{Deserialize, Serialize};

use crate::runtime::error::ResumeCommand;
use crate::runtime::event::PermissionReply;

/// Permission decision outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Ask,
    Deny,
}

/// Permission rule with ordered pattern matching.
#[derive(Clone, Debug)]
pub struct PermissionRule {
    pub action: PermissionDecision,
    pub patterns: Vec<String>,
}

impl PermissionRule {
    pub fn new(action: PermissionDecision, patterns: Vec<String>) -> Self {
        Self { action, patterns }
    }
}

/// Permission policy that evaluates rules in order.
#[derive(Clone, Debug, Default)]
pub struct PermissionPolicy {
    pub rules: Vec<PermissionRule>,
}

impl PermissionPolicy {
    pub fn new(rules: Vec<PermissionRule>) -> Self {
        Self { rules }
    }

    pub fn decide(&self, permission: &str) -> PermissionDecision {
        for rule in &self.rules {
            if rule.patterns.iter().any(|pattern| matches_pattern(pattern, permission)) {
                return rule.action;
            }
        }
        PermissionDecision::Allow
    }
}

/// Gate interface to allow custom evaluators.
pub trait PermissionGate: Send + Sync {
    fn decide(&self, permission: &str) -> PermissionDecision;
}

impl PermissionGate for PermissionPolicy {
    fn decide(&self, permission: &str) -> PermissionDecision {
        self.decide(permission)
    }
}

#[derive(Default)]
struct PermissionOverrides {
    once: std::collections::HashSet<String>,
    always: std::collections::HashSet<String>,
    reject: std::collections::HashSet<String>,
}

impl PermissionOverrides {
    fn decide(&mut self, permission: &str) -> Option<PermissionDecision> {
        if self.reject.contains(permission) {
            return Some(PermissionDecision::Deny);
        }
        if self.always.contains(permission) {
            return Some(PermissionDecision::Allow);
        }
        if self.once.remove(permission) {
            return Some(PermissionDecision::Allow);
        }
        None
    }

    fn apply_reply(&mut self, permission: &str, reply: crate::runtime::event::PermissionReply) {
        match reply {
            crate::runtime::event::PermissionReply::Once => {
                self.once.insert(permission.to_string());
            }
            crate::runtime::event::PermissionReply::Always => {
                self.always.insert(permission.to_string());
            }
            crate::runtime::event::PermissionReply::Reject => {
                self.reject.insert(permission.to_string());
            }
        }
    }
}

/// Mutable permission session that can accept runtime replies.
pub struct PermissionSession {
    base: PermissionPolicy,
    overrides: std::sync::Mutex<PermissionOverrides>,
}

impl PermissionSession {
    pub fn new(base: PermissionPolicy) -> Self {
        Self {
            base,
            overrides: std::sync::Mutex::new(PermissionOverrides::default()),
        }
    }

    pub fn snapshot(&self) -> PermissionSnapshot {
        let overrides = self.overrides.lock().unwrap();
        PermissionSnapshot {
            once: overrides.once.iter().cloned().collect(),
            always: overrides.always.iter().cloned().collect(),
            reject: overrides.reject.iter().cloned().collect(),
        }
    }

    pub fn restore(&self, snapshot: PermissionSnapshot) {
        let mut overrides = self.overrides.lock().unwrap();
        overrides.once = snapshot.once.into_iter().collect();
        overrides.always = snapshot.always.into_iter().collect();
        overrides.reject = snapshot.reject.into_iter().collect();
    }

    pub fn apply_reply(&self, permission: &str, reply: crate::runtime::event::PermissionReply) {
        let mut overrides = self.overrides.lock().unwrap();
        overrides.apply_reply(permission, reply);
    }

    pub fn apply_resume(
        &self,
        permission: &str,
        command: &ResumeCommand,
    ) -> Option<PermissionReply> {
        let reply = parse_permission_reply(&command.value)?;
        self.apply_reply(permission, reply.clone());
        Some(reply)
    }
}

impl PermissionGate for PermissionSession {
    fn decide(&self, permission: &str) -> PermissionDecision {
        let mut overrides = self.overrides.lock().unwrap();
        if let Some(decision) = overrides.decide(permission) {
            return decision;
        }
        self.base.decide(permission)
    }
}

fn matches_pattern(pattern: &str, permission: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return permission.starts_with(prefix);
    }
    permission == pattern
}

/// Permission request payload used in interrupts.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PermissionRequest {
    pub permission: String,
    pub patterns: Vec<String>,
}

/// Serializable snapshot of runtime permission replies.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PermissionSnapshot {
    pub once: Vec<String>,
    pub always: Vec<String>,
    pub reject: Vec<String>,
}

/// Persistence adapter for permission sessions.
pub trait PermissionStore: Send + Sync {
    fn load(&self, session_id: &str) -> Option<PermissionSnapshot>;
    fn save(&self, session_id: &str, snapshot: PermissionSnapshot);
}

/// In-memory permission store for tests/local use.
#[derive(Default)]
pub struct InMemoryPermissionStore {
    snapshots: std::sync::Mutex<std::collections::HashMap<String, PermissionSnapshot>>,
}

impl InMemoryPermissionStore {
    pub fn new() -> Self {
        Self {
            snapshots: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl PermissionStore for InMemoryPermissionStore {
    fn load(&self, session_id: &str) -> Option<PermissionSnapshot> {
        self.snapshots.lock().unwrap().get(session_id).cloned()
    }

    fn save(&self, session_id: &str, snapshot: PermissionSnapshot) {
        self.snapshots
            .lock()
            .unwrap()
            .insert(session_id.to_string(), snapshot);
    }
}

fn parse_permission_reply(value: &serde_json::Value) -> Option<PermissionReply> {
    match value {
        serde_json::Value::String(value) => parse_reply_str(value),
        serde_json::Value::Object(map) => map
            .get("reply")
            .and_then(|reply| reply.as_str())
            .and_then(parse_reply_str),
        _ => None,
    }
}

fn parse_reply_str(value: &str) -> Option<PermissionReply> {
    match value.to_lowercase().as_str() {
        "once" => Some(PermissionReply::Once),
        "always" => Some(PermissionReply::Always),
        "reject" | "deny" => Some(PermissionReply::Reject),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PermissionDecision,
        PermissionGate,
        PermissionPolicy,
        PermissionRequest,
        PermissionRule,
        PermissionSession,
        PermissionSnapshot,
        PermissionStore,
        InMemoryPermissionStore,
    };
    use crate::runtime::error::ResumeCommand;
    use crate::runtime::event::PermissionReply;

    #[test]
    fn permission_policy_uses_first_match() {
        let policy = PermissionPolicy::new(vec![
            PermissionRule::new(PermissionDecision::Deny, vec!["file:*".to_string()]),
            PermissionRule::new(PermissionDecision::Ask, vec!["file:read".to_string()]),
            PermissionRule::new(PermissionDecision::Allow, vec!["*".to_string()]),
        ]);

        assert_eq!(policy.decide("file:read"), PermissionDecision::Deny);
        assert_eq!(policy.decide("net:fetch"), PermissionDecision::Allow);
    }

    #[test]
    fn permission_policy_matches_prefix_wildcards() {
        let policy = PermissionPolicy::new(vec![PermissionRule::new(
            PermissionDecision::Ask,
            vec!["tool:*".to_string()],
        )]);

        assert_eq!(policy.decide("tool:grep"), PermissionDecision::Ask);
        assert_eq!(policy.decide("tools:grep"), PermissionDecision::Allow);
    }

    #[test]
    fn permission_policy_defaults_to_allow() {
        let policy = PermissionPolicy::new(vec![]);
        assert_eq!(policy.decide("file:write"), PermissionDecision::Allow);
    }

    #[test]
    fn permission_session_once_consumes_override() {
        let base = PermissionPolicy::new(vec![PermissionRule::new(
            PermissionDecision::Ask,
            vec!["tool:echo".to_string()],
        )]);
        let session = PermissionSession::new(base);
        session.apply_reply("tool:echo", PermissionReply::Once);

        assert_eq!(session.decide("tool:echo"), PermissionDecision::Allow);
        assert_eq!(session.decide("tool:echo"), PermissionDecision::Ask);
    }

    #[test]
    fn permission_session_always_allows() {
        let base = PermissionPolicy::new(vec![PermissionRule::new(
            PermissionDecision::Ask,
            vec!["tool:echo".to_string()],
        )]);
        let session = PermissionSession::new(base);
        session.apply_reply("tool:echo", PermissionReply::Always);

        assert_eq!(session.decide("tool:echo"), PermissionDecision::Allow);
        assert_eq!(session.decide("tool:echo"), PermissionDecision::Allow);
    }

    #[test]
    fn permission_session_reject_denies() {
        let base = PermissionPolicy::new(vec![]);
        let session = PermissionSession::new(base);
        session.apply_reply("tool:rm", PermissionReply::Reject);

        assert_eq!(session.decide("tool:rm"), PermissionDecision::Deny);
    }

    #[test]
    fn permission_session_applies_resume_command() {
        let base = PermissionPolicy::new(vec![PermissionRule::new(
            PermissionDecision::Ask,
            vec!["tool:echo".to_string()],
        )]);
        let session = PermissionSession::new(base);
        let command = ResumeCommand::new("once");
        let reply = session.apply_resume("tool:echo", &command);

        assert_eq!(reply, Some(PermissionReply::Once));
        assert_eq!(session.decide("tool:echo"), PermissionDecision::Allow);
        assert_eq!(session.decide("tool:echo"), PermissionDecision::Ask);
    }

    #[test]
    fn permission_request_roundtrip() {
        let request = PermissionRequest {
            permission: "tool:echo".to_string(),
            patterns: vec!["tool:echo".to_string()],
        };
        let json = serde_json::to_value(&request).expect("serialize");
        let decoded: PermissionRequest = serde_json::from_value(json).expect("deserialize");
        assert_eq!(request, decoded);
    }

    #[test]
    fn permission_snapshot_roundtrip() {
        let snapshot = PermissionSnapshot {
            once: vec!["tool:echo".to_string()],
            always: vec!["tool:read".to_string()],
            reject: vec!["tool:rm".to_string()],
        };
        let json = serde_json::to_value(&snapshot).expect("serialize");
        let decoded: PermissionSnapshot = serde_json::from_value(json).expect("deserialize");
        assert_eq!(snapshot, decoded);
    }

    #[test]
    fn permission_session_snapshot_restore() {
        let base = PermissionPolicy::new(vec![PermissionRule::new(
            PermissionDecision::Ask,
            vec!["tool:echo".to_string()],
        )]);
        let session = PermissionSession::new(base);
        session.apply_reply("tool:echo", PermissionReply::Once);
        session.apply_reply("tool:read", PermissionReply::Always);
        session.apply_reply("tool:rm", PermissionReply::Reject);

        let snapshot = session.snapshot();
        let restored = PermissionSession::new(PermissionPolicy::new(vec![]));
        restored.restore(snapshot);

        assert_eq!(restored.decide("tool:echo"), PermissionDecision::Allow);
        assert_eq!(restored.decide("tool:echo"), PermissionDecision::Allow);
        assert_eq!(restored.decide("tool:read"), PermissionDecision::Allow);
        assert_eq!(restored.decide("tool:rm"), PermissionDecision::Deny);
    }

    #[test]
    fn permission_store_roundtrip() {
        let store = InMemoryPermissionStore::new();
        let snapshot = PermissionSnapshot {
            once: vec!["tool:echo".to_string()],
            always: vec!["tool:read".to_string()],
            reject: vec!["tool:rm".to_string()],
        };

        assert!(store.load("s1").is_none());
        store.save("s1", snapshot.clone());
        assert_eq!(store.load("s1"), Some(snapshot));
    }
}
