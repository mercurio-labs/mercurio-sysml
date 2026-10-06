// Generated from resolved javac policy trees by tools/export_pilot_definition_defaults.py.
// Handwritten dependencies supply individual, owned-end count, metadata and library lookup.
pub(super) fn inputs(kind: &str) -> Option<(bool, bool)> {
    match kind {
        "ActionDefinition" => Some((true, false)),
        "AllocationDefinition" => Some((true, true)),
        "AnalysisCaseDefinition" => Some((true, false)),
        "AttributeDefinition" => Some((false, false)),
        "CalculationDefinition" => Some((true, false)),
        "CaseDefinition" => Some((true, false)),
        "ConcernDefinition" => Some((true, false)),
        "ConjugatedPortDefinition" => Some((false, false)),
        "ConnectionDefinition" => Some((true, true)),
        "ConstraintDefinition" => Some((true, false)),
        "Definition" => Some((false, false)),
        "EnumerationDefinition" => Some((false, false)),
        "FlowDefinition" => Some((true, true)),
        "InterfaceDefinition" => Some((true, true)),
        "ItemDefinition" => Some((true, false)),
        "MetadataDefinition" => Some((true, false)),
        "OccurrenceDefinition" => Some((true, false)),
        "PartDefinition" => Some((true, false)),
        "PortDefinition" => Some((false, false)),
        "RenderingDefinition" => Some((true, false)),
        "RequirementDefinition" => Some((true, false)),
        "StateDefinition" => Some((true, false)),
        "UseCaseDefinition" => Some((true, false)),
        "VerificationCaseDefinition" => Some((true, false)),
        "ViewDefinition" => Some((true, false)),
        "ViewpointDefinition" => Some((true, false)),
        _ => None,
    }
}
#[allow(unused_parens)]
pub(super) fn names(kind: &str, individual: bool, owned_end_count: usize) -> Option<Vec<&'static str>> {
    let mut names = Vec::new();
    match kind {
        "ActionDefinition" => {
            names.push("Actions::Action");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "AllocationDefinition" => {
            names.push(if (owned_end_count != 2) { "Allocations::Allocation" } else { "Allocations::Allocation" });
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "AnalysisCaseDefinition" => {
            names.push("AnalysisCases::AnalysisCase");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "AttributeDefinition" => {
            names.push("Base::DataValue");
        },
        "CalculationDefinition" => {
            names.push("Calculations::Calculation");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "CaseDefinition" => {
            names.push("Cases::Case");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "ConcernDefinition" => {
            names.push("Requirements::ConcernCheck");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "ConjugatedPortDefinition" => {
            names.push("Ports::Port");
        },
        "ConnectionDefinition" => {
            names.push(if (owned_end_count != 2) { "Connections::Connection" } else { "Connections::BinaryConnection" });
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "ConstraintDefinition" => {
            names.push("Constraints::ConstraintCheck");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "Definition" => {
            names.push("Base::Anything");
        },
        "EnumerationDefinition" => {
            names.push("Base::DataValue");
        },
        "FlowDefinition" => {
            names.push(if (owned_end_count != 2) { "Flows::MessageAction" } else { "Flows::Message" });
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "InterfaceDefinition" => {
            names.push(if (owned_end_count != 2) { "Interfaces::Interface" } else { "Interfaces::BinaryInterface" });
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "ItemDefinition" => {
            names.push("Items::Item");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "MetadataDefinition" => {
            names.push("Metadata::MetadataItem");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "OccurrenceDefinition" => {
            names.push("Occurrences::Occurrence");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "PartDefinition" => {
            names.push("Parts::Part");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "PortDefinition" => {
            names.push("Ports::Port");
        },
        "RenderingDefinition" => {
            names.push("Views::Rendering");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "RequirementDefinition" => {
            names.push("Requirements::RequirementCheck");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "StateDefinition" => {
            names.push("States::StateAction");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "UseCaseDefinition" => {
            names.push("UseCases::UseCase");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "VerificationCaseDefinition" => {
            names.push("VerificationCases::VerificationCase");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "ViewDefinition" => {
            names.push("Views::View");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        "ViewpointDefinition" => {
            names.push("Views::ViewpointCheck");
            if individual {
            names.push("Occurrences::Life");
            }
        },
        _ => return None,
    }
    Some(names)
}
