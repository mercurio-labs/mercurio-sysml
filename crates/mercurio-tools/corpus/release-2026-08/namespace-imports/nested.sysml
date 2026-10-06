package P { part def C; }
package User { public import P::*; protected import P::C; private import all P::**; }
