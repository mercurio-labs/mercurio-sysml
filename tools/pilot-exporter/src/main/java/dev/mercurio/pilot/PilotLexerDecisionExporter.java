package dev.mercurio.pilot;
import java.nio.file.*;
import java.util.*;
import java.lang.reflect.*;
import org.antlr.runtime.*;
import com.google.gson.GsonBuilder;

/** Export upstream DFA arrays and exhaustively evaluate LA(1)-only special transitions. */
public final class PilotLexerDecisionExporter {
    static Object field(DFA dfa, String name) throws Exception {
        var field = DFA.class.getDeclaredField(name); field.setAccessible(true);
        Object value = field.get(dfa);
        if(value instanceof char[] chars) { var result = new int[chars.length]; for(int i=0;i<chars.length;i++) result[i]=chars[i]; return result; }
        return value;
    }
    static Map<String,Object> decision(Lexer lexer, String[] tokenNames) throws Exception {
        DFA dfa = null;
        for(var field : lexer.getClass().getDeclaredFields()) {
            if(!DFA.class.isAssignableFrom(field.getType())) continue;
            field.setAccessible(true);
            if(dfa != null) throw new IllegalStateException("Multiple token decisions");
            dfa = (DFA)field.get(lexer);
        }
        if(dfa == null) throw new IllegalStateException("Missing token decision");
        String description = dfa.getDescription();
        String prefix="1:1: Tokens : ( ", suffix=" );";
        if(!description.startsWith(prefix) || !description.endsWith(suffix)) throw new IllegalStateException("Unknown decision labels");
        var labels = description.substring(prefix.length(),description.length()-suffix.length()).split(" \\| ");
        for(String label:labels) if(!label.matches("T__\\d+|RULE_[A-Z_]+")) throw new IllegalStateException("Invalid token label: "+label);
        var result = new TreeMap<String,Object>();
        for(String name : List.of("eot","eof","min","max","accept","special","transition")) result.put(name,field(dfa,name));
        result.put("labels", labels);
        var keywords = new TreeMap<String,String>();
        for(String label:labels) if(label.startsWith("T__")) {
            String spelling=tokenNames[Integer.parseInt(label.substring(3))];
            if(!spelling.startsWith("\'") || !spelling.endsWith("\'") || spelling.contains("\\")) throw new IllegalStateException("Unsupported keyword spelling: "+spelling);
            keywords.put(label,spelling.substring(1,spelling.length()-1));
        }
        result.put("keywords",keywords);
        var specialRows = new TreeMap<Integer,Object>();
        for(short state : (short[])field(dfa,"special")) {
            if(state < 0 || specialRows.containsKey((int)state)) continue;
            var ranges = new ArrayList<Object>();
            int start=-1, previous=-2;
            for(int code=-1;code<=65536;code++) {
                int target=-2;
                if(code<=65535) {
                    final int unit=code;
                    var input=(IntStream)Proxy.newProxyInstance(IntStream.class.getClassLoader(),new Class<?>[]{IntStream.class},(proxy,method,values)->{
                        if(method.getName().equals("LA") && (int)values[0]==1) return unit;
                        if(method.getName().equals("index")) return 0; // RecognitionException diagnostics only.
                        throw new IllegalStateException("Nonlocal special transition: "+method.getName());
                    });
                    try { target=dfa.specialStateTransition(state,input); } catch(NoViableAltException error) { target=-1; }
                }
                if(code==-1) { previous=target; continue; }
                if(target!=previous) { ranges.add(List.of(start,code-1,previous)); start=code; previous=target; }
            }
            specialRows.put((int)state,ranges);
        }
        var controls = new ArrayList<Object>();
        var sources = new LinkedHashSet<String>();
        for(int code=0;code<128;code++) {
            sources.add(Character.toString((char)code));
            sources.add("//"+Character.toString((char)code)+"tail");
            sources.add("//*"+Character.toString((char)code)+"*/ tail");
        }
        sources.addAll(List.of("", " \t\r\n", "//* note */ P { // line\r\n}", "//* note */ part def A; }", "// x\r}", "1e", "1e+2", "package", "part", "classifier", "\u00a0", "\uffff", "😀"));
        for(String keyword:keywords.values()) sources.addAll(List.of(keyword, keyword+"x", keyword+" ", keyword+keyword));
        for(String source:sources) {
            var row=new TreeMap<String,Object>(); row.put("source",source);
            try { row.put("label",labels[dfa.predict(new ANTLRStringStream(source))-1]); }
            catch(RecognitionException error) { row.put("label",null); }
            controls.add(row);
        }
        result.put("prediction_controls",controls);
        result.put("special_ranges",specialRows);
        result.put("schema","dev.mercurio.lexer-decision.v1");
        return result;
    }
    public static void main(String[] args) throws Exception {
        String[] tokenNames = org.omg.sysml.xtext.parser.antlr.internal.InternalSysMLParser.tokenNames;
        var result=decision(new org.omg.sysml.xtext.parser.antlr.internal.InternalSysMLLexer(),tokenNames);
        result.put("kerml",decision(new org.omg.kerml.xtext.parser.antlr.internal.InternalKerMLLexer(),org.omg.kerml.xtext.parser.antlr.internal.InternalKerMLParser.tokenNames));
        var tokenControls=new ArrayList<Object>();
        for(String source:List.of("0", "12x", "12e3", "12E-3x", "1e", "1e+", "1e-x", "1.2e3", ".5", "1..2", "1...2", "1.e2", "12e3e4", ".", "..")) {
            var actual=new org.omg.sysml.xtext.parser.antlr.internal.InternalSysMLLexer(new ANTLRStringStream(source)) {
                public int errors=0;
                @Override public void reportError(RecognitionException e) { errors++; }
            };
            var tokens=new ArrayList<Object>();
            for(Token token=actual.nextToken();token.getType()!=Token.EOF;token=actual.nextToken()) {
                var row=new TreeMap<String,Object>();
                row.put("name",tokenNames[token.getType()]); row.put("text",token.getText()); tokens.add(row);
            }
            var row=new TreeMap<String,Object>(); row.put("source",source); row.put("tokens",tokens); row.put("errors",actual.errors); tokenControls.add(row);
        }
        result.put("numeric_token_controls",tokenControls);
        var bodyControls = new ArrayList<Object>();
        for(String rule:List.of("DECIMAL_VALUE","EXP_VALUE","ID","UNRESTRICTED_NAME","STRING_VALUE","REGULAR_COMMENT","ML_NOTE","SL_NOTE","WS")) {
            var bodySources = new LinkedHashSet<String>();
            bodySources.addAll(List.of("", "0", "123x", "12e-3x", "12e+", "alpha_9!", "//", "//x\rrest", "//x\r\nrest", "/*open", "//*open", "/*a*/b*/", "//*a*/b*/", " \t\r\nx", "\u00a0", "\"😀\"tail", "'😀'tail", "\"bad\\x\"", "'bad\\x'"));
            for(int code=0;code<128;code++) {
                String c=Character.toString((char)code);
                bodySources.add(switch(rule) {
                    case "DECIMAL_VALUE" -> c+"9x";
                    case "EXP_VALUE" -> "1e"+c+"9x";
                    case "ID" -> c+"a9";
                    case "UNRESTRICTED_NAME" -> "'"+c+"'x";
                    case "STRING_VALUE" -> "\""+c+"\"x";
                    case "REGULAR_COMMENT" -> "/*"+c+"*/x";
                    case "ML_NOTE" -> "//*"+c+"*/x";
                    case "SL_NOTE" -> "//x"+c+"tail";
                    default -> c+" \tX";
                });
            }
            for(String source:bodySources) {
                var input=new ANTLRStringStream(source);
                var actual=new org.omg.sysml.xtext.parser.antlr.internal.InternalSysMLLexer(input);
                var row=new TreeMap<String,Object>(); row.put("rule",rule); row.put("source",source);
                try {
                    actual.getClass().getMethod("mRULE_"+rule).invoke(actual);
                    row.put("length_utf16",input.index());
                } catch(InvocationTargetException error) {
                    if(!(error.getCause() instanceof RecognitionException)) throw error;
                    row.put("length_utf16",null);
                }
                bodyControls.add(row);
            }
        }
        result.put("terminal_body_controls",bodyControls);
        Files.writeString(Path.of(args[0]),new GsonBuilder().serializeNulls().create().toJson(result)+"\n");
    }
}
