import copy
import importlib.util
import pathlib
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / 'summarize.py'
spec = importlib.util.spec_from_file_location('summarize', SCRIPT)
assert spec is not None and spec.loader is not None
summarize = importlib.util.module_from_spec(spec)
spec.loader.exec_module(summarize)


class EvidenceTest(unittest.TestCase):
    def test_retained_artifacts(self):
        result = summarize.report()
        self.assertEqual(list(result['campaigns']), ['d1-d6-1', 'd8ab-1', 'follow-1', 'step1-3'])
        current = result['campaigns']['follow-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3916, 'protocol-refusal': 1283,
                                             'decoder-refusal': 401, 'mismatch': 0})
        self.assertEqual(current['families']['spec'], {'cases': 800, 'match': 684, 'protocol-refusal': 67,
                                                       'decoder-refusal': 49, 'mismatch': 0})
        current = result['campaigns']['d8ab-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3954, 'protocol-refusal': 1258,
                                             'decoder-refusal': 388, 'mismatch': 0})
        self.assertEqual(current['families']['batch'], {'cases': 800, 'match': 425, 'protocol-refusal': 375,
                                                        'decoder-refusal': 0, 'mismatch': 0})
        settled = result['campaigns']['d1-d6-1']
        self.assertEqual(settled['totals'], {'cases': 5600, 'match': 4107, 'protocol-refusal': 1124,
                                             'decoder-refusal': 369, 'mismatch': 0})
        self.assertEqual(settled['families']['lens'], {'cases': 800, 'match': 571, 'protocol-refusal': 229,
                                                       'decoder-refusal': 0, 'mismatch': 0})
        campaign = result['campaigns']['step1-3']
        self.assertEqual(campaign['totals']['cases'], 5600)
        self.assertEqual(campaign['totals']['mismatch'], 42)
        self.assertEqual(campaign['families']['lens']['mismatch'], 42)
        self.assertEqual(list(result['mutations']), ['mutations-d1-d6', 'mutations-d8ab', 'mutations-fixes2'])
        current = result['mutations']['mutations-d8ab']
        self.assertEqual(current['total'], 210)
        self.assertEqual(current['outcomes'], {'error': 15, 'killed-by-vector': 28, 'rejected-by-build': 165, 'survived': 2})
        self.assertEqual(current['survivors'], ['finish-json-hexdigit', 'last-spec-tests-conds'])
        self.assertEqual(current['guards']['d8-of-not-root-skipped'], 'killed-by-vector')
        earlier = result['mutations']['mutations-d1-d6']
        self.assertEqual(earlier['total'], 201)
        self.assertEqual(earlier['outcomes'], {'error': 15, 'killed-by-vector': 27, 'rejected-by-build': 157, 'survived': 2})
        mutation = result['mutations']['mutations-fixes2']
        self.assertEqual(mutation['total'], 195)
        self.assertEqual(mutation['outcomes'], {'error': 16, 'killed-by-vector': 27, 'rejected-by-build': 146, 'survived': 6})

    def test_category_sum_rejected(self):
        summary = summarize.read(summarize.CAMPAIGNS / 'step1-3/summary.json')
        summary['counts']['lens']['match'] += 1
        with self.assertRaises(ValueError):
            summarize.totals(summary)

    def test_new_d1_example_rejected(self):
        path = summarize.CAMPAIGNS / 'step1-3'
        summary = summarize.read(path / 'summary.json')
        data = summarize.read(path / 'd1-mismatches.json')
        reference = copy.deepcopy(data['examples'])
        data['examples']['raw']['input']['text'] = ['different']
        with self.assertRaises(ValueError):
            summarize.validate_cases(summary, data, reference)

    def test_infrastructure_failure_rejected(self):
        artifact = summarize.read(summarize.MUTATIONS / 'mutations-fixes2/summary.json')
        artifact['mutants'][0]['commands'][0]['signal'] = 'SIGTERM'
        with self.assertRaises(ValueError):
            summarize.mutation_report(artifact)
        artifact['mutants'][0]['commands'][0]['signal'] = None
        artifact['mutants'][0]['commands'][0]['status'] = 'timeout'
        with self.assertRaises(ValueError):
            summarize.mutation_report(artifact)


if __name__ == '__main__':
    unittest.main()
