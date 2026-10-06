package SingleType_constraint_inherited_redundant { constraint def A; constraint def B :> A; constraint a: A; constraint b: B; constraint c subsets a, b; }
