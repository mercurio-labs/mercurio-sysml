import unittest
from analyze_definition_query_trace import analyze

def event(t,phase,owner="x",field="r"):
    return f"definition query: elapsed_ms={t} phase={phase} owner={owner} kind=Type field={field} elements=12 succeeded=None"

class TraceTests(unittest.TestCase):
    def test_nested_time_and_local_repeated_keys(self):
        rows=[event(10,"ReferenceStarted"),event(11,"GeneralTypesStarted","g"),event(12,"LibraryPathStarted","","Base::Anything"),
            event(15,"LibraryPathFinished","","Base::Anything"),event(20,"GeneralTypesFinished","g"),event(21,"GeneralTypesStarted","g"),
            event(25,"GeneralTypesFinished","g"),event(30,"ReferenceFinished"),event(31,"ReferenceStarted"),
            event(32,"GeneralTypesStarted","g"),event(33,"GeneralTypesFinished","g"),event(35,"ReferenceFinished")]
        result=analyze("\n".join(rows));self.assertEqual(result["completed_reference_attempts"],2)
        self.assertEqual(result["stages"]["GeneralTypes"]["inclusive_ms"],14)
        self.assertEqual(result["stages"]["GeneralTypes"]["exclusive_ms"],11)
        self.assertEqual(len(result["repeated_keys_within_one_reference"]),1)
        self.assertEqual(result["repeated_keys_within_one_reference"][0]["calls"],2)
        self.assertEqual(result["inflight"],[])
    def test_timeout_retains_open_frames(self):
        result=analyze(event(1,"ReferenceStarted")+"\n"+event(2,"GeneralTypesStarted","g"))
        self.assertEqual(len(result["inflight"]),2);self.assertEqual(result["completed_reference_attempts"],0)
        self.assertEqual(result["qualification"],"not_assessed")
    def test_mismatched_boundary_rejected(self):
        with self.assertRaises(ValueError): analyze(event(1,"ReferenceStarted")+"\n"+event(2,"GeneralTypesFinished"))
    def test_nonmonotonic_clock_rejected(self):
        with self.assertRaises(ValueError): analyze(event(2,"ReferenceStarted")+"\n"+event(1,"ReferenceFinished"))
    def test_nested_same_identity_is_lifo(self):
        result=analyze("\n".join([event(1,"ScopeMembershipsStarted"),event(2,"ScopeMembershipsStarted"),
            event(3,"ScopeMembershipsFinished"),event(5,"ScopeMembershipsFinished")]))
        self.assertEqual(result["stages"]["ScopeMemberships"]["inclusive_ms"],5)
        self.assertEqual(result["stages"]["ScopeMemberships"]["exclusive_ms"],4)

if __name__=="__main__": unittest.main()
