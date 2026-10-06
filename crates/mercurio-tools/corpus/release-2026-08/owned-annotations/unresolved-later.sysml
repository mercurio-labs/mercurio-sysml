package P {
 part def A; part def B;
 dependency d from A to B {
 comment about A, Missing /* Bad */
 }
}
