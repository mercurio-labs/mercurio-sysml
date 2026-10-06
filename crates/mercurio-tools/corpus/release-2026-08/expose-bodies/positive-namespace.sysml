package P {
 part vehicle {
  part engine;
 }
 view v {
  expose vehicle::* {
   doc /* Scope documentation */
   comment /* Scope comment */
   rep language "text" /* Scope representation */
  }
 }
}
