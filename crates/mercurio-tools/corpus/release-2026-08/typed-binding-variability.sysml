package TypedBindingVariability {
    part def A;
    part def B;
    connection def AB { end item a : A; end b : B; }
    part def Scope {
        part a;
        part b;
        binding bound : AB bind a = b;
    }
}
