// Generated from guarded resolved getter programs and Ecore ranges.
pub(super) fn rule(field: &str) -> Option<(&'static str,&'static str,&'static str)> { match field {
 "trigger_action" => Some(("trigger","AcceptActionUsage","org.omg.sysml.delegate.setting.TransitionUsage_triggerAction_SettingDelegate")),
 "guard_expression" => Some(("guard","Expression","org.omg.sysml.delegate.setting.TransitionUsage_guardExpression_SettingDelegate")),
 "effect_action" => Some(("effect","ActionUsage","org.omg.sysml.delegate.setting.TransitionUsage_effectAction_SettingDelegate")),
 "succession" => Some(("","Succession","org.omg.sysml.delegate.setting.TransitionUsage_succession_SettingDelegate")),
 _ => None,
} }
