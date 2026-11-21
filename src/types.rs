//! Core types and representations for Rustad verdict evaluation.

/// Verdict returned when evaluating a URL against filter rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Verdict {
    /// No matching rules found; request should proceed.
    Pass = 0,
    /// Request matched a blocking rule; request should be canceled or blocked.
    Block = 1,
    /// Request matched an explicit whitelist/exception rule; request must be allowed.
    Allow = 2,
}

impl Verdict {
    /// Returns `true` if the verdict indicates the request should be blocked.
    #[inline]
    #[must_use]
    pub fn is_blocked(self) -> bool {
        matches!(self, Verdict::Block)
    }

    /// Returns `true` if the verdict indicates the request is explicitly allowed.
    #[inline]
    #[must_use]
    pub fn is_allowed(self) -> bool {
        matches!(self, Verdict::Allow)
    }

    /// Returns `true` if the request passed through without matching rules.
    #[inline]
    #[must_use]
    pub fn is_pass(self) -> bool {
        matches!(self, Verdict::Pass)
    }
}
