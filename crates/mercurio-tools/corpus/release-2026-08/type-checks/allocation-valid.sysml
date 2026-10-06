package TypeCheck_allocation_valid { part def P; allocation def T { end left: P; end right: P; } part a: P; part b: P; allocation value: T allocate a to b; }
