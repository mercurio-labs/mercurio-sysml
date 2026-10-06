// Generated pinned resolved shared Connector selector and succession maps.
// Additional decision/merge/context/end algorithms are separate dependencies.
pub(super) fn default_for(kind:&str,owned_ends:usize,structure:bool)->Option<&'static str> {
    match (kind,owned_ends==2,structure) {
        ("Succession",false,false) => Some("Links::links"),
        ("Succession",true,false) => Some("Occurrences::happensBeforeLinks"),
        ("Succession",false,true) => Some("Objects::linkObjects"),
        ("Succession",true,true) => Some("Occurrences::happensBeforeLinks"),
        ("SuccessionAsUsage",false,false) => Some("Occurrences::happensBeforeLinks"),
        ("SuccessionAsUsage",true,false) => Some("Occurrences::happensBeforeLinks"),
        ("SuccessionAsUsage",false,true) => Some("Objects::objects"),
        ("SuccessionAsUsage",true,true) => Some("Occurrences::happensBeforeLinks"),
        _ => None,
    }
}
