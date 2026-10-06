package TypeCheck_interface_valid { port def P; interface def T { end port left: P; end port right: P; } port a: P; port b: P; interface value: T connect a to b; }
