# SysML v2 release sample audit

Scope: all release samples.

Acceptance parity: **FAIL**.
Full specification conformance and semantic equivalence are not established.

| Corpus | Files | Native pass | Pilot pass | Pilot only accepts | Native only accepts | Infrastructure failures |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| kerml/src/examples | 58 | 26 | 58 | 32 | 0 | 0 |
| sysml/src/examples | 96 | 44 | 96 | 52 | 0 | 0 |
| sysml/src/training | 100 | 63 | 100 | 37 | 0 | 0 |
| sysml/src/validation | 56 | 23 | 56 | 33 | 0 | 0 |
| TOTAL | 310 | 156 | 310 | 154 | 0 | 0 |

Native uses its shipped baseline and strict parser/resolver/lowering APIs. Pilot uses the release's textual standard library and CheckMode.ALL.
Cases are folder-scoped with explicit dependencies; validation fixtures and alternate examples may intentionally fail.
Warnings do not fail Pilot. Crashes, timeouts, and missing results are infrastructure failures, never parity.
All raw diagnostics, hashes, source sets, and process logs are retained beside this report.

## Differences

- kerml/src/examples/Association Examples/ProductSelection_N_ary.kerml: native=error, Pilot=ok. unresolved import `inCart`
- kerml/src/examples/Association Examples/ProductSelection_OwnedEnds.kerml: native=error, Pilot=ok. unresolved import `selectedProduct::selectedProducts`
- kerml/src/examples/Association Examples/ProductSelection_UnownedEnds.kerml: native=error, Pilot=ok. unresolved import `selectedProduct::selectedProduct1`
- kerml/src/examples/KerML Spec Annex A Examples/A-2-ModelingInstances.kerml: native=error, Pilot=ok. unresolved import `Atoms::atom`
- kerml/src/examples/KerML Spec Annex A Examples/A-3-5-TimingForStructures.kerml: native=error, Pilot=ok. unresolved import `OneToOneConnectorsExecution::MyWheel`
- kerml/src/examples/Packet Example/Packets.kerml: native=error, Pilot=ok. unresolved import `Time::DateTime`
- kerml/src/examples/Simple Tests/ArgumentResolution.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Associations.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Behaviors.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Circular.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Classes.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Classifications.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Classifiers.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Comments.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Conjugation.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Connectors.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Dependencies.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Expansion.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Expressions.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/FeatureChains.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/FeatureInheritance.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Features.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Filtering.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Imports.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Inheritance.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Inverses.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/MetadataTest.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Redefinition.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Scoping.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/TextualRepresentation.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Simple Tests/Types.kerml: native=error, Pilot=ok. expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`
- kerml/src/examples/Variable Feature Examples/Enhancements/TimeVaryingSteps.kerml: native=error, Pilot=ok. unresolved import `merge`
- sysml/src/examples/Arrowhead Framework Example/AHFNorwayTopics.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/examples/Individuals Examples/JohnIndividualExample.sysml: native=error, Pilot=ok. unresolved redefinition target `presidentOfUS`
- sysml/src/examples/Interaction Sequencing Examples/ServerSequenceModel.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Interaction Sequencing Examples/ServerSequenceModelOutside.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Interaction Sequencing Examples/ServerSequenceOutsideRealization-2.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Interaction Sequencing Examples/ServerSequenceOutsideRealization-3.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Interaction Sequencing Examples/ServerSequenceRealization-2.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Interaction Sequencing Examples/ServerSequenceRealization-3.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Requirements Examples/RequirementDerivationExample.sysml: native=error, Pilot=ok. unresolved reference target `req1`
- sysml/src/examples/Simple Tests/ActionTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/AliasTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/AllocationTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/AnalysisTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/AssignmentTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/CalculationTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/CommentTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/ConjugationTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/ConnectionTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/ConstraintTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/ControlNodeTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/DecisionTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/DefaultValueTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/DependencyTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/EnumerationTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/FeaturePathTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/ImportTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/IndividualTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/InterfaceTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/ItemTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/MetadataTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/MultiplicityTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/OccurrenceTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/ParameterTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/PartTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/RequirementTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/RootPackageTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/StateTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/StructuredControlTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/TextualRepresentationTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/TradeStudyTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/UseCaseTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/VariabilityTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/VerificationTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Simple Tests/ViewTest.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/State Space Representation Examples/CartSample.sysml: native=error, Pilot=ok. unresolved type `CartInput`
- sysml/src/examples/State Space Representation Examples/EVSample.sysml: native=error, Pilot=ok. unresolved type `VehicleInput`
- sysml/src/examples/Timeslice and Snapshot Examples/TimeVaryingAttribute.sysml: native=error, Pilot=ok. unresolved redefinition target `localClock::currentTime`
- sysml/src/examples/Variability Examples/VehicleVariabilityModel.sysml: native=error, Pilot=ok. unresolved redefinition target `cylinder`
- sysml/src/examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Vehicle Example/VehicleDefinitions.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Vehicle Example/VehicleIndividuals.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/examples/Vehicle Example/VehicleUsages.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/training/12. Binding Connectors/Binding Connectors Example-1.sysml: native=error, Pilot=ok. duplicate emitted KIR id `binding.Binding Connectors Example-1.vehicle.tank.fuelTankPort`
- sysml/src/training/13. Flows/Flow Definition Example.sysml: native=error, Pilot=ok. unresolved redefinition target `payload`
- sysml/src/training/13. Flows/Flow Usage Example.sysml: native=error, Pilot=ok. unresolved reference target `tankAssy::fuelTankPort::fuelSupply`
- sysml/src/training/14. Action Definitions/Action Shorthand Example.sysml: native=error, Pilot=ok. unresolved expression name `TakePicture::scene`
- sysml/src/training/17. Control/Decision Example.sysml: native=error, Pilot=ok. unresolved specialization `decide`
- sysml/src/training/17. Control/Fork Join Example.sysml: native=error, Pilot=ok. unresolved specialization `fork`
- sysml/src/training/17. Control/Merge Example.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/18. Action Performance/Action Performance Example.sysml: native=error, Pilot=ok. unresolved specialization `takePhoto::focus`
- sysml/src/training/19. Terminate Actions/Terminate Actions Example-1.sysml: native=error, Pilot=ok. unresolved specialization `fork`
- sysml/src/training/19. Terminate Actions/Terminate Actions Example-2.sysml: native=error, Pilot=ok. duplicate emitted KIR id `feature.Terminate Actions Example-2.terminateProcessing.processor`
- sysml/src/training/20. Assignment Actions/Assignment Example.sysml: native=error, Pilot=ok. expected expression
- sysml/src/training/21. Asynchronous Messaging/Messaging Example.sysml: native=error, Pilot=ok. unresolved type `scene`
- sysml/src/training/21. Asynchronous Messaging/Messaging with Ports.sysml: native=error, Pilot=ok. unresolved type `scene`
- sysml/src/training/23. State Definitions/State Definition Example-1.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/23. State Definitions/State Definition Example-2.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/24. States/State Actions.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/24. States/State Decomposition-1.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/24. States/State Decomposition-2.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/25. Transitions/Change and Time Triggers.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/25. Transitions/Transition Actions.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/27. Occurrences/Interaction Realization-1.sysml: native=error, Pilot=ok. unresolved type `Driver`
- sysml/src/training/27. Occurrences/Interaction Realization-2.sysml: native=error, Pilot=ok. unresolved type `Driver`
- sysml/src/training/27. Occurrences/Message Payload Example.sysml: native=error, Pilot=ok. unresolved expression name `fuelCommandMessage::fuelCommand`
- sysml/src/training/28. Individuals/Individuals and Snapshots Example.sysml: native=error, Pilot=ok. unresolved specialization `vehicle_1_t0`
- sysml/src/training/28. Individuals/Individuals and Time Slices.sysml: native=error, Pilot=ok. unresolved specialization `Person`
- sysml/src/training/29. Expressions/Car Mass Rollup Example 1.sysml: native=error, Pilot=ok. unresolved redefinition target `simpleMass`
- sysml/src/training/29. Expressions/Car Mass Rollup Example 2.sysml: native=error, Pilot=ok. unresolved redefinition target `simpleMass`
- sysml/src/training/29. Expressions/MassRollup1.sysml: native=error, Pilot=ok. unresolved expression name `simpleMass`
- sysml/src/training/29. Expressions/MassRollup2.sysml: native=error, Pilot=ok. unresolved expression name `simpleMass`
- sysml/src/training/31. Constraints/Time Constraints.sysml: native=error, Pilot=ok. unresolved expression name `normal::done`
- sysml/src/training/32. Requirements/Requirement Satisfaction.sysml: native=error, Pilot=ok. duplicate emitted KIR id `satisfy.Requirement Satisfaction.Vehicle c1 Design Context.satisfy`
- sysml/src/training/35. Use Cases/Use Case Usage Example.sysml: native=error, Pilot=ok. unresolved specialization `start`
- sysml/src/training/38. Allocation/Allocation Definition Example.sysml: native=error, Pilot=ok. unresolved specialization `providePower::generateTorque`
- sysml/src/training/38. Allocation/Allocation Usage Example.sysml: native=error, Pilot=ok. unresolved specialization `providePower::generateTorque`
- sysml/src/training/39. Metadata/Metadata Example-2.sysml: native=error, Pilot=ok. unresolved type `ISQ::AccelerationValue`
- sysml/src/training/41. Language Extension/Model Library Example.sysml: native=error, Pilot=ok. duplicate emitted KIR id `occurrence.Model Library Example.Scenario.situations`
- sysml/src/training/41. Language Extension/User Keyword Example.sysml: native=error, Pilot=ok. unresolved expression name `LevelEnum::high`
- sysml/src/validation/01-Parts Tree/1a-Parts Tree.sysml: native=error, Pilot=ok. unresolved redefinition target `Vehicle::mass`
- sysml/src/validation/01-Parts Tree/1c-Parts Tree Redefinition.sysml: native=error, Pilot=ok. unresolved redefinition target `Vehicle::mass`
- sysml/src/validation/02-Parts Interconnection/2a-Parts Interconnection.sysml: native=error, Pilot=ok. unresolved reference target `rearAxle::leftHalfAxle::axleToWheelPort`
- sysml/src/validation/03-Function-based Behavior/3a-Function-based Behavior-1.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/03-Function-based Behavior/3a-Function-based Behavior-2.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/03-Function-based Behavior/3a-Function-based Behavior-3.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/03-Function-based Behavior/3c-Function-based Behavior-structure mod-1.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/03-Function-based Behavior/3c-Function-based Behavior-structure mod-2.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/03-Function-based Behavior/3c-Function-based Behavior-structure mod-3.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/03-Function-based Behavior/3d-Function-based Behavior-item.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/03-Function-based Behavior/3e-Function-based Behavior-item.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/04-Functional Allocation/4a-Functional Allocation.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/05-State-based Behavior/5-State-based Behavior-1.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/validation/05-State-based Behavior/5-State-based Behavior-1a.sysml: native=error, Pilot=ok. expected `;` or body terminator after transition
- sysml/src/validation/05-State-based Behavior/5-State-based Behavior-2.sysml: native=error, Pilot=ok. expected `from` before flow source
- sysml/src/validation/07-Variant Configuration/7a-Variant Configuration - General Concept.sysml: native=error, Pilot=ok. unresolved redefinition target `part1`
- sysml/src/validation/07-Variant Configuration/7a1-Variant Configuration - General Concept-a.sysml: native=error, Pilot=ok. unresolved redefinition target `part3`
- sysml/src/validation/08-Requirements/8-Requirements.sysml: native=error, Pilot=ok. unresolved redefinition target `drivePwrPort`
- sysml/src/validation/09-Verification/9-Verification-simplified.sysml: native=error, Pilot=ok. unresolved redefinition target `massRequirement`
- sysml/src/validation/10-Analysis and Trades/10a-Analysis.sysml: native=error, Pilot=ok. expected expression
- sysml/src/validation/10-Analysis and Trades/10b-Trade-off Among Alternative Configurations.sysml: native=error, Pilot=ok. expected expression
- sysml/src/validation/10-Analysis and Trades/10c-Fuel Economy Analysis.sysml: native=error, Pilot=ok. expected expression
- sysml/src/validation/10-Analysis and Trades/10d-Dynamics Analysis.sysml: native=error, Pilot=ok. expected expression
- sysml/src/validation/12-Dependency Relationships/12b-Allocation-1.sysml: native=error, Pilot=ok. unresolved specialization `providePower::generateTorque`
- sysml/src/validation/12-Dependency Relationships/12b-Allocation.sysml: native=error, Pilot=ok. unresolved specialization `providePower::generateTorque`
- sysml/src/validation/13-Model Containment/13a-Model Containment.sysml: native=error, Pilot=ok. unresolved import `VehicleSubsystems`
- sysml/src/validation/14-Language Extensions/14a-Language Extensions.sysml: native=error, Pilot=ok. unresolved redefinition target `annotatedElement`
- sysml/src/validation/14-Language Extensions/14c-Language Extensions.sysml: native=error, Pilot=ok. unresolved type `SysML::ItemDefinition`
- sysml/src/validation/15-Properties-Values-Expressions/15_08-Range Restriction.sysml: native=error, Pilot=ok. unresolved specialization `PlaneAngleValue`
- sysml/src/validation/15-Properties-Values-Expressions/15_13-Discretely Sampled Function Value.sysml: native=error, Pilot=ok. unresolved redefinition target `definition`
- sysml/src/validation/15-Properties-Values-Expressions/15_19-Materials with Properties.sysml: native=error, Pilot=ok. unresolved expression name `isq::M`
- sysml/src/validation/15-Properties-Values-Expressions/15_19a-Materials with Properties.sysml: native=error, Pilot=ok. unresolved expression name `isq::M`
- sysml/src/validation/18-Use Case/18-Use Case.sysml: native=error, Pilot=ok. unresolved redefinition target `done`
