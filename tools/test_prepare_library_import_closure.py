import unittest
from prepare_library_import_closure import select

def row(name, package, imports=()):
    return {'release_path': name, 'packages': [package], 'imports': list(imports)}
def declaration(owner, target):
    return {'owner_package': owner, 'target_segments': target}

class LibraryImportClosureTests(unittest.TestCase):
    def test_external_prefix_does_not_match_owner_and_cycles_terminate(self):
        inventory={'resources':[row('p',['P'],[declaration(['P'],['Q','value'])]),row('q',['Q'],[declaration(['Q'],['P','value'])])]}
        result=select(inventory,['P'])
        self.assertEqual(result['resources'],['p','q'])
        self.assertEqual(result['package_prefix_edges'][0]['provider'],'q')
        self.assertEqual(result['unresolved_import_prefixes'],[])
    def test_nested_relative_package_is_selected_before_global(self):
        inventory={'resources':[row('p',['P'],[declaration(['P'],['N','value'])]),row('nested',['P','N']),row('global',['N'])]}
        self.assertEqual(select(inventory,['P'])['resources'],['nested','p'])
    def test_missing_targets_are_explicit_and_duplicate_exports_reject(self):
        inventory={'resources':[row('p',['P'],[declaration(['P'],['Missing','x'])])]}
        self.assertEqual(len(select(inventory,['P'])['unresolved_import_prefixes']),1)
        inventory['resources'].append(row('duplicate',['P']))
        with self.assertRaisesRegex(ValueError,'ambiguous'):
            select(inventory,['P'])
    def test_filter_target_keeps_nested_explicit_import_edges(self):
        inventory={'resources':[row('p',['P'],[{'owner_package':['P'],'target_segments':[],'implicit_filter_target':True},declaration(['P'],['Q','x'])]),row('q',['Q'])]}
        self.assertEqual(select(inventory,['P'])['resources'],['p','q'])
