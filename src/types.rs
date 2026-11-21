// ==============================================================================
// RUSTAD CORE TYPES (src/types.rs)
// Simple, zero-complexity enums and representations
// ==============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Verdict {
    Pass = 0,
    Block = 1,
    Allow = 2,
}

impl Verdict {
    #[inline]
    pub fn is_blocked(self) -> bool {
        matches!(self, Verdict::Block)
    }

    #[inline]
    pub fn is_allowed(self) -> bool {
        matches!(self, Verdict::Allow)
    }

    #[inline]
    pub fn is_pass(self) -> bool {
        matches!(self, Verdict::Pass)
    }
}
