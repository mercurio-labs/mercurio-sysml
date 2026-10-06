// Generated from resolved State/Action inherited predicates and imported default map.
// Handwritten dependencies: canonical ownership/typing, selected Ecore invocation and entry/exit membership.
pub(super) struct Inputs<'a> {pub composite:bool,pub structure:bool,pub data:bool,pub owner_kind:&'a str,pub owner_class:bool,pub owner_structure:bool,pub portion:&'a str,pub entry_exit:bool,pub exclusive_state:bool,pub substate:bool}
impl Inputs<'_> {fn owner_is(&self,kind:&str)->bool {super::metaclass_conforms(self.owner_kind,kind)}}
#[allow(unused_parens)]
pub(super) fn names(c:&Inputs<'_>)->Option<Vec<&'static str>> {let mut roles=Vec::new();
    roles.push("base");
    if c.data {
        roles.push("dataValue");
    }
    if c.structure {
        roles.push(if c.composite && (c.owner_is("Structure") || (c.owner_is("Feature") && c.owner_structure)) { "subobject" } else { "object" });
    } else {
        if ((c.composite && (c.owner_is("Class") || (c.owner_is("Feature") && c.owner_class))) || (c.composite && c.owner_is("OccurrenceUsage"))) && (!((c.composite && (!c.entry_exit)) && (c.owner_is("ActionDefinition") || c.owner_is("ActionUsage")))) {
            roles.push("suboccurrence");
        }
    }
    if c.portion == "snapshot" {
        roles.push("snapshot");
    } else {
        if c.portion == "timeslice" {
            roles.push("timeslice");
        }
    }
    if (if c.exclusive_state { "exclusiveState" } else { (if c.substate { "substate" } else { (if (c.composite && (!c.entry_exit)) && (c.owner_is("ActionDefinition") || c.owner_is("ActionUsage")) { "subaction" } else { (if c.composite && (c.owner_is("PartDefinition") || c.owner_is("PartUsage")) { "ownedAction" } else { "" }) }) }) }) != "" {
        roles.push((if c.exclusive_state { "exclusiveState" } else { (if c.substate { "substate" } else { (if (c.composite && (!c.entry_exit)) && (c.owner_is("ActionDefinition") || c.owner_is("ActionUsage")) { "subaction" } else { (if c.composite && (c.owner_is("PartDefinition") || c.owner_is("PartUsage")) { "ownedAction" } else { "" }) }) }) }));
    }
    if c.composite && (c.owner_is("Structure") || (c.owner_is("Feature") && c.owner_structure)) {
        roles.push("ownedPerformance");
    } else {
        if (c.owner_is("Behavior") || c.owner_is("Step")) && c.composite {
            roles.push("subperformance");
        } else {
            if c.owner_is("Behavior") || c.owner_is("Step") {
                roles.push("enclosedPerformance");
            }
        }
    }
let mut result=Vec::new();for role in roles {let name=match role {
"base"=>"States::stateActions",
"dataValue"=>"Base::dataValues",
"enclosedPerformance"=>"Performances::Performance::enclosedPerformances",
"exclusiveState"=>"States::StateAction::exclusiveStates",
"object"=>"Objects::objects",
"ownedAction"=>"Parts::Part::ownedStates",
"ownedPerformance"=>"Objects::Object::ownedPerformances",
"snapshot"=>"Occurrences::Occurrence::snapshots",
"subaction"=>"Actions::Action::subactions",
"subobject"=>"Objects::Object::subobjects",
"suboccurrence"=>"Occurrences::Occurrence::suboccurrences",
"subperformance"=>"Performances::Performance::subperformances",
"substate"=>"States::StateAction::substates",
"timeslice"=>"Occurrences::Occurrence::timeSlices",
_=>return None,};if !result.contains(&name) {result.push(name);}}Some(result)}
