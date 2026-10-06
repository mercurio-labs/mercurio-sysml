package KernelConnectors {
    class Node;
    assoc struct Pair { end feature left : Node; end feature right : Node; }
    feature scope : Scope;
    redefinition scope.a redefines scope.b;
    subset scope.c subsets scope.a;
    class Scope {
        feature a : Node;
        feature b : Node;
        feature c : Node;
        connector <pair> binary from left references a to right ::> b;
        connector ternary (a, b, c);
        connector typedLink : Pair from a to b;
        abstract connector bound = binary;
        connector withBody { end feature references a; end feature references b; }
        binding equal of a = b;
        binding typedBinding : Pair of a = b;
        binding a = b;
        succession priorNext first a then b;
        succession typedOrder : Pair first a then b;
        succession a then b;
        connector decorated from a to b { feature nested : Node; }
        expr calculate {
            in operand : Node;
            return result : Node;
            binding result.portionOf = operand;
        }
    }
}
