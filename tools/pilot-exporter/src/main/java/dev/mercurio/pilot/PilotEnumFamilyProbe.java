package dev.mercurio.pilot;
import com.google.gson.*;
import com.google.inject.Injector;
import java.nio.file.*;
import java.util.*;
import java.io.StringReader;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.IParser;
import org.eclipse.emf.common.util.Enumerator;
import org.antlr.runtime.*;
/** Independent selected-rule enum results plus actual grammar caller models. */
public final class PilotEnumFamilyProbe {
 public static void main(String[]args)throws Exception {
  org.omg.sysml.logic.SysMLLogicStandaloneSetup.doSetup();
  org.eclipse.emf.ecore.EPackage.Registry.INSTANCE.put("https://www.omg.org/spec/SysML/20250201",org.omg.sysml.lang.sysml.SysMLPackage.eINSTANCE);
  var cases=new ArrayList<Object>();var models=new ArrayList<Object>();
  for(boolean kerml:List.of(true,false)){
   Injector injector=kerml?new org.omg.kerml.xtext.KerMLStandaloneSetup().createInjectorAndDoEMFRegistration():new org.omg.sysml.xtext.SysMLStandaloneSetup().createInjectorAndDoEMFRegistration();
   var grammar=injector.getInstance(IGrammarAccess.class).getGrammar();
   var ruleRows=new ArrayList<EnumRule>();
   for(var rule:GrammarUtil.allRules(grammar))if(rule instanceof EnumRule e)ruleRows.add(e);
   for(var rule:ruleRows){
    var literals=new ArrayList<EnumLiteralDeclaration>();var it=rule.eAllContents();while(it.hasNext())if(it.next() instanceof EnumLiteralDeclaration e)literals.add(e);
    for(var declaration:literals){
     String spelling=declaration.getLiteral().getValue();
     String[] carrier=switch(rule.getName()){
      case "FilterPackageMemberVisibility" -> new String[]{"FilterPackageMember","[true]","visibility"};
      case "VisibilityIndicator" -> new String[]{kerml?"NonFeatureMember":"PackageMember",spelling+" package P;","visibility"};
      case "FeatureDirection" -> new String[]{kerml?"Feature":"AttributeUsage",spelling+(kerml?" feature f;":" attribute f;"),"direction"};
      case "PortionKind" -> new String[]{"PartUsage",spelling+" part p;","portionKind"};
      case "TriggerFeatureKind" -> new String[]{"TriggerActionMember","accept x","kind"};
      case "GuardFeatureKind" -> new String[]{"GuardExpressionMember","if true","kind"};
      case "EffectFeatureKind" -> new String[]{"EffectBehaviorMember","do","kind"};
      case "RequirementConstraintKind" -> new String[]{"RequirementConstraintMember",spelling+" constraint c;","kind"};
      case "FramedConcernKind" -> new String[]{"FramedConcernMember","frame concern c;","kind"};
      case "RequirementVerificationKind" -> new String[]{"RequirementVerificationMember","verify requirement r;","kind"};
      case "ExposeVisibilityKind" -> new String[]{"Expose","expose P::*;","visibility"};
      default -> throw new IllegalStateException(rule.getName());
     };
     var carriers=new ArrayList<String[]>();carriers.add(carrier);
     if(rule.getName().equals("VisibilityIndicator"))carriers.add(new String[]{"Import",spelling+" import P::*;","visibility"});
     if(rule.getName().equals("PortionKind")){
      carriers.add(new String[]{"PortionUsage",spelling+" p;","portionKind"});
      carriers.add(new String[]{"MergeNode",spelling+" merge m;","portionKind"});
     }
     for(var modelCarrier:carriers){
      var parsed=injector.getInstance(IParser.class).parse((ParserRule)GrammarUtil.findRuleForName(grammar,modelCarrier[0]),new StringReader(modelCarrier[1]));
      if(parsed.hasSyntaxErrors())throw new IllegalStateException("Invalid enum carrier "+modelCarrier[0]+": "+modelCarrier[1]+" "+parsed.getSyntaxErrors());
      var object=parsed.getRootASTElement();var feature=object.eClass().getEStructuralFeature(modelCarrier[2]);var modelValue=(Enumerator)object.eGet(feature,false);
      if(modelValue==null)throw new IllegalStateException("Absent carrier enum "+modelCarrier[1]);
      models.add(Map.of("language",grammar.getName(),"rule",rule.getName(),"token",spelling,"carrier_rule",modelCarrier[0],"source",modelCarrier[1],"kind",object.eClass().getName(),"field",modelCarrier[2],"literal",modelValue.getLiteral(),"value",modelValue.getValue(),"primary",modelCarrier==carrier));
     }
     for(String polarity:List.of("positive","negative","boundary")){
      String source=polarity.equals("positive")?spelling:polarity.equals("negative")?"'"+spelling+"'":spelling+"x";
      String stem=kerml?"org.omg.kerml.xtext":"org.omg.sysml.xtext";
      String name=kerml?"KerML":"SysML";
      var lexer=(TokenSource)Class.forName(stem+".parser.antlr.internal.Internal"+name+"Lexer").getConstructor(CharStream.class).newInstance(new ANTLRStringStream(source));
      var tokenDefs=injector.getInstance(org.eclipse.xtext.parser.antlr.ITokenDefProvider.class);
      var tokens=new org.eclipse.xtext.parser.antlr.XtextTokenStream(lexer,tokenDefs);
      var access=injector.getInstance(Class.forName(stem+".services."+name+"GrammarAccess"));
      var parser=Class.forName(stem+".parser.antlr.internal.Internal"+name+"Parser").getConstructor(TokenStream.class,access.getClass()).newInstance(tokens,access);
      injector.injectMembers(parser);
      var internal=(org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser)parser;
      internal.setTokenTypeMap(tokenDefs.getTokenDefMap());
      internal.setSyntaxErrorProvider(injector.getInstance(org.eclipse.xtext.parser.antlr.ISyntaxErrorMessageProvider.class));
      var current=org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser.class.getDeclaredField("currentNode");current.setAccessible(true);current.set(parser,internal.getNodeModelBuilder().newRootNode(source));
      Enumerator value=null;boolean accepted=false;
      try{value=(Enumerator)parser.getClass().getMethod("rule"+rule.getName()).invoke(parser);accepted=((BaseRecognizer)parser).getNumberOfSyntaxErrors()==0&&tokens.LA(1)==Token.EOF&&value!=null;}catch(java.lang.reflect.InvocationTargetException e){if(!(e.getCause() instanceof RecognitionException))throw e;}
      var row=new TreeMap<String,Object>();row.put("language",grammar.getName());row.put("rule",rule.getName());row.put("token",spelling);row.put("source",source);row.put("polarity",polarity);row.put("accepted",accepted);
      if(accepted){row.put("literal",value.getLiteral());row.put("value",value.getValue());}
      cases.add(row);
     }
    }
   }
  }
  Files.writeString(Path.of(args[0]),new GsonBuilder().setPrettyPrinting().create().toJson(Map.of("controls",cases,"model_controls",models))+"\n");
 }
}
