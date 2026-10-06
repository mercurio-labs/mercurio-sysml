// Generated reviewed policy. Ownership projection/execution are handwritten.
pub(super) const MEMBERSHIP: &str = "Membership";
pub(super) fn insert(empty: bool, first_parameter: bool) -> bool { empty || first_parameter }
pub(super) fn eligible(feature: bool, parameter: bool, transition: bool, connector: bool, message: bool) -> bool { feature && !parameter && !transition && (!connector || message) }
