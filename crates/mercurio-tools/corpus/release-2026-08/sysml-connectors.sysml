package SysmlConnectors {
    connection def Pair { end left; end right; }
    part scope {
        part a;
        part b;
        binding namedBinding bind a = b;
        binding typedBinding : Pair bind a = b;
        binding bind a = b;
        succession namedOrder first a then b;
        succession typedOrder : Pair first a then b;
        succession first a then b;
        succession first b then a;
    }
}
