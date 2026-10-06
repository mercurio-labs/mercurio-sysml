package TextualRepresentations {
    item def C {
        assert constraint checked {
            rep <ocl> inOCL language "ocl" /* self.x > 0.0 */
        }
        rep language "plain" /*  preserve spaces  */
        language "alf" /* first line
 * second line
 */
    }
}
