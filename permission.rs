//! Permission evaluation primitives for tool/operation gating.

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

fn matches_pattern(pattern: &str, permission: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return permission.starts_with(prefix);
    }
    permission == pattern
}

#[cfg(test)]
mod tests {
    use super::{PermissionDecision, PermissionPolicy, PermissionRule};

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
}
