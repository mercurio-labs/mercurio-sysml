// Generated from pinned resolved selector trees and actual adapter dispatch.
pub(super) const PARTICIPANT_DEFAULT: &str = "Links::Link::participant";
// Ecore ownership, contextual predicates and redefinition algorithms are handwritten dependencies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Relevant { None, Ends, Constructor, Parameters }
pub(super) fn relevant(kind: &str, has_owner: bool, is_end: bool, constructor_result: bool, is_parameter: bool) -> Option<Relevant> {
    Some(match kind {
        "AllocationUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "AssertConstraintUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "AttributeUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "BindingConnector" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "BindingConnectorAsUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "ConnectionUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "Connector" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "ConstraintUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "EnumerationUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "EventOccurrenceUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "Feature" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "Flow" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "FlowEnd" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "FlowUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "InterfaceUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "ItemUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "MetadataFeature" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "MetadataUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "Multiplicity" => Relevant::None,
        "MultiplicityRange" => Relevant::None,
        "OccurrenceUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "PartUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "PortUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "Step" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "Succession" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "SuccessionAsUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "SuccessionFlow" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "SuccessionFlowUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "Usage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        "ViewUsage" => if !has_owner { Relevant::None } else { if is_end { Relevant::Ends } else { if constructor_result { Relevant::Constructor } else { if is_parameter { Relevant::Parameters } else { Relevant::None } } } },
        _ => return None,
    })
}
pub(super) fn no_additional_members(kind: &str) -> bool {
    matches!(kind,
        "ActionDefinition" | "AllocationDefinition" | "AnalysisCaseDefinition" | "Association" | "AssociationStructure" | "AttributeDefinition" | "Behavior" | "BindingConnector" | "BooleanExpression" | "CalculationDefinition" | "CaseDefinition" | "Class" | "Classifier" | "ConcernDefinition" | "ConjugatedPortDefinition" | "ConnectionDefinition" | "Connector" | "ConstraintDefinition" | "DataType" | "Definition" | "EnumerationDefinition" | "Expression" | "Feature" | "Flow" | "FlowDefinition" | "FlowEnd" | "Function" | "Interaction" | "InterfaceDefinition" | "Invariant" | "ItemDefinition" | "LiteralBoolean" | "LiteralExpression" | "LiteralInfinity" | "LiteralInteger" | "LiteralRational" | "LiteralString" | "Metaclass" | "MetadataAccessExpression" | "MetadataDefinition" | "MetadataFeature" | "Multiplicity" | "MultiplicityRange" | "NullExpression" | "OccurrenceDefinition" | "PartDefinition" | "PayloadFeature" | "PortDefinition" | "Predicate" | "RenderingDefinition" | "RequirementDefinition" | "StateDefinition" | "Step" | "Structure" | "Succession" | "SuccessionAsUsage" | "SuccessionFlow" | "Type" | "UseCaseDefinition" | "VerificationCaseDefinition" | "ViewDefinition" | "ViewpointDefinition"
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EndSource { Owned, Effective }
pub(super) fn end_source(same_owner: bool) -> EndSource { if same_owner { EndSource::Owned } else { EndSource::Effective } }
pub(super) fn end_strategy_supported(kind: &str) -> bool {
    matches!(kind,
        "AllocationUsage" | "AssertConstraintUsage" | "AttributeUsage" | "BindingConnector" | "BindingConnectorAsUsage" | "ConnectionUsage" | "Connector" | "ConstraintUsage" | "EnumerationUsage" | "EventOccurrenceUsage" | "Feature" | "Flow" | "FlowEnd" | "FlowUsage" | "InterfaceUsage" | "ItemUsage" | "MetadataFeature" | "MetadataUsage" | "Multiplicity" | "MultiplicityRange" | "OccurrenceUsage" | "PartUsage" | "PortUsage" | "Step" | "Succession" | "SuccessionAsUsage" | "SuccessionFlow" | "SuccessionFlowUsage" | "Usage" | "ViewUsage"
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ParameterSelection { Empty, Result, Parameters }
pub(super) fn parameter_selection(has_type: bool, is_result: bool, has_result: bool) -> ParameterSelection {
if has_type {
if is_result {
let result_parameter = has_result;
if result_parameter {
return ParameterSelection::Result;
}
} else {
return ParameterSelection::Parameters;
}
}
return ParameterSelection::Empty;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ParameterCollection { Owned, Effective }
pub(super) fn parameter_collection(same_owner: bool) -> ParameterCollection { if same_owner { ParameterCollection::Owned } else { ParameterCollection::Effective } }
pub(super) fn ignored_parameter(is_result: bool) -> bool { is_result }
pub(super) fn parameter_strategy_supported(kind: &str) -> bool {
    matches!(kind,
        "AllocationUsage" | "AssertConstraintUsage" | "AttributeUsage" | "BindingConnector" | "BindingConnectorAsUsage" | "ConnectionUsage" | "Connector" | "ConstraintUsage" | "EnumerationUsage" | "EventOccurrenceUsage" | "Feature" | "Flow" | "FlowEnd" | "FlowUsage" | "InterfaceUsage" | "ItemUsage" | "MetadataFeature" | "MetadataUsage" | "Multiplicity" | "MultiplicityRange" | "OccurrenceUsage" | "PartUsage" | "PortUsage" | "Step" | "Succession" | "SuccessionAsUsage" | "SuccessionFlow" | "SuccessionFlowUsage" | "Usage" | "ViewUsage"
    )
}
pub(super) fn ignored_parameter_supported(kind: &str) -> bool {
    matches!(kind,
        "AcceptActionUsage" | "ActionUsage" | "AllocationUsage" | "AnalysisCaseUsage" | "AssertConstraintUsage" | "AssignmentActionUsage" | "AttributeUsage" | "BindingConnector" | "BindingConnectorAsUsage" | "BooleanExpression" | "CalculationUsage" | "CaseUsage" | "CollectExpression" | "ConcernUsage" | "ConnectionUsage" | "Connector" | "ConstraintUsage" | "ConstructorExpression" | "DecisionNode" | "EnumerationUsage" | "EventOccurrenceUsage" | "ExhibitStateUsage" | "Expression" | "Feature" | "FeatureChainExpression" | "FeatureReferenceExpression" | "Flow" | "FlowEnd" | "FlowUsage" | "ForLoopActionUsage" | "ForkNode" | "IfActionUsage" | "IncludeUseCaseUsage" | "IndexExpression" | "InterfaceUsage" | "Invariant" | "InvocationExpression" | "ItemUsage" | "JoinNode" | "LiteralBoolean" | "LiteralExpression" | "LiteralInfinity" | "LiteralInteger" | "LiteralRational" | "LiteralString" | "MergeNode" | "MetadataAccessExpression" | "MetadataFeature" | "MetadataUsage" | "Multiplicity" | "MultiplicityRange" | "NullExpression" | "OccurrenceUsage" | "OperatorExpression" | "PartUsage" | "PayloadFeature" | "PerformActionUsage" | "PortUsage" | "ReferenceUsage" | "RenderingUsage" | "RequirementUsage" | "SatisfyRequirementUsage" | "SelectExpression" | "SendActionUsage" | "StateUsage" | "Step" | "Succession" | "SuccessionAsUsage" | "SuccessionFlow" | "SuccessionFlowUsage" | "TerminateActionUsage" | "TransitionUsage" | "TriggerInvocationExpression" | "Usage" | "UseCaseUsage" | "VerificationCaseUsage" | "ViewUsage" | "ViewpointUsage" | "WhileLoopActionUsage"
    )
}
pub(super) fn owning_type_featuring_supported(kind: &str) -> bool {
    matches!(kind, "BindingConnector" | "BooleanExpression" | "CollectExpression" | "Connector" | "ConstructorExpression" | "Expression" | "Feature" | "FeatureChainExpression" | "FeatureReferenceExpression" | "Flow" | "FlowEnd" | "IndexExpression" | "Invariant" | "InvocationExpression" | "LiteralBoolean" | "LiteralExpression" | "LiteralInfinity" | "LiteralInteger" | "LiteralRational" | "LiteralString" | "MetadataAccessExpression" | "MetadataFeature" | "Multiplicity" | "MultiplicityRange" | "NullExpression" | "OperatorExpression" | "PayloadFeature" | "SelectExpression" | "Step" | "Succession" | "SuccessionFlow" | "TriggerInvocationExpression")
}
pub(super) fn owning_type_featuring_uses_variability(kind: &str) -> bool {
    matches!(kind, "AcceptActionUsage" | "ActionUsage" | "AllocationUsage" | "AnalysisCaseUsage" | "AssertConstraintUsage" | "AssignmentActionUsage" | "AttributeUsage" | "BindingConnectorAsUsage" | "CalculationUsage" | "CaseUsage" | "ConcernUsage" | "ConnectionUsage" | "ConstraintUsage" | "DecisionNode" | "EnumerationUsage" | "EventOccurrenceUsage" | "ExhibitStateUsage" | "FlowUsage" | "ForLoopActionUsage" | "ForkNode" | "IfActionUsage" | "IncludeUseCaseUsage" | "InterfaceUsage" | "ItemUsage" | "JoinNode" | "MergeNode" | "MetadataUsage" | "OccurrenceUsage" | "PartUsage" | "PerformActionUsage" | "PortUsage" | "ReferenceUsage" | "RenderingUsage" | "RequirementUsage" | "SatisfyRequirementUsage" | "SendActionUsage" | "StateUsage" | "SuccessionAsUsage" | "SuccessionFlowUsage" | "TerminateActionUsage" | "TransitionUsage" | "Usage" | "UseCaseUsage" | "VerificationCaseUsage" | "ViewUsage" | "ViewpointUsage" | "WhileLoopActionUsage")
}
