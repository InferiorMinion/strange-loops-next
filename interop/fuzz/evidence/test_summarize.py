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
        self.assertEqual(list(result['campaigns']), ['by-1', 'by-collect-1', 'by-shape-1', 'collect-limit-1', 'd1-d6-1', 'd8ab-1', 'follow-2', 'issue-7-1', 'keyed-1', 'keyed-slots-1', 'keyed-slots-2', 'sets-1', 'shape-follow-1', 'shape-under-1', 'shape-under-2', 'step1-3', 'under-1'])
        current = result['campaigns']['collect-limit-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3638, 'protocol-refusal': 1579,
                                             'decoder-refusal': 383, 'mismatch': 0})
        self.assertEqual(current['families']['spec'], {'cases': 800, 'match': 686, 'protocol-refusal': 58,
                                                       'decoder-refusal': 56, 'mismatch': 0})
        current = result['campaigns']['keyed-slots-2']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3629, 'protocol-refusal': 1590,
                                             'decoder-refusal': 381, 'mismatch': 0})
        self.assertEqual(current['families']['spec'], {'cases': 800, 'match': 695, 'protocol-refusal': 53,
                                                       'decoder-refusal': 52, 'mismatch': 0})
        current = result['campaigns']['keyed-slots-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3607, 'protocol-refusal': 1600,
                                             'decoder-refusal': 393, 'mismatch': 0})
        self.assertEqual(current['families']['spec'], {'cases': 800, 'match': 693, 'protocol-refusal': 53,
                                                       'decoder-refusal': 54, 'mismatch': 0})
        current = result['campaigns']['keyed-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3585, 'protocol-refusal': 1614,
                                             'decoder-refusal': 401, 'mismatch': 0})
        self.assertEqual(current['families']['spec'], {'cases': 800, 'match': 683, 'protocol-refusal': 52,
                                                       'decoder-refusal': 65, 'mismatch': 0})
        self.assertEqual(current['families']['scenario'], {'cases': 800, 'match': 703, 'protocol-refusal': 0,
                                                           'decoder-refusal': 97, 'mismatch': 0})
        current = result['campaigns']['sets-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3614, 'protocol-refusal': 1576,
                                             'decoder-refusal': 410, 'mismatch': 0})
        self.assertEqual(current['families']['record'], {'cases': 800, 'match': 55, 'protocol-refusal': 718,
                                                         'decoder-refusal': 27, 'mismatch': 0})
        self.assertEqual(current['families']['batch'], {'cases': 800, 'match': 202, 'protocol-refusal': 586,
                                                        'decoder-refusal': 12, 'mismatch': 0})
        current = result['campaigns']['under-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3776, 'protocol-refusal': 1477,
                                             'decoder-refusal': 347, 'mismatch': 0})
        self.assertEqual(current['families']['batch'], {'cases': 800, 'match': 257, 'protocol-refusal': 531,
                                                        'decoder-refusal': 12, 'mismatch': 0})
        current = result['campaigns']['issue-7-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3788, 'protocol-refusal': 1437,
                                             'decoder-refusal': 375, 'mismatch': 0})
        self.assertEqual(current['families']['spec'], {'cases': 800, 'match': 707, 'protocol-refusal': 43,
                                                       'decoder-refusal': 50, 'mismatch': 0})
        current = result['campaigns']['by-collect-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3801, 'protocol-refusal': 1421,
                                             'decoder-refusal': 378, 'mismatch': 0})
        self.assertEqual(current['families']['spec'], {'cases': 800, 'match': 710, 'protocol-refusal': 48,
                                                       'decoder-refusal': 42, 'mismatch': 0})
        current = result['campaigns']['by-shape-1']
        self.assertEqual(current['totals'], {'cases': 5600, 'match': 3778, 'protocol-refusal': 1426,
                                             'decoder-refusal': 396, 'mismatch': 0})
        self.assertEqual(current['families']['spec'], {'cases': 800, 'match': 698, 'protocol-refusal': 53,
                                                       'decoder-refusal': 49, 'mismatch': 0})
        grouped = result['campaigns']['by-1']
        self.assertEqual(grouped['totals'], {'cases': 5600, 'match': 3924, 'protocol-refusal': 1256,
                                             'decoder-refusal': 420, 'mismatch': 0})
        merged = result['campaigns']['shape-follow-1']
        self.assertEqual(merged['totals'], {'cases': 5600, 'match': 3807, 'protocol-refusal': 1434,
                                            'decoder-refusal': 359, 'mismatch': 0})
        self.assertEqual(merged['families']['batch'], {'cases': 800, 'match': 307, 'protocol-refusal': 481,
                                                       'decoder-refusal': 12, 'mismatch': 0})
        follow = result['campaigns']['follow-2']
        self.assertEqual(follow['totals'], {'cases': 5600, 'match': 3908, 'protocol-refusal': 1324,
                                            'decoder-refusal': 368, 'mismatch': 0})
        newest = result['campaigns']['shape-under-2']
        self.assertEqual(newest['totals'], {'cases': 5600, 'match': 3785, 'protocol-refusal': 1401,
                                            'decoder-refusal': 414, 'mismatch': 0})
        self.assertEqual(newest['families']['batch'], {'cases': 800, 'match': 329, 'protocol-refusal': 459,
                                                       'decoder-refusal': 12, 'mismatch': 0})
        latest = result['campaigns']['shape-under-1']
        self.assertEqual(latest['totals'], {'cases': 5600, 'match': 3803, 'protocol-refusal': 1391,
                                            'decoder-refusal': 406, 'mismatch': 0})
        self.assertEqual(latest['families']['record'], {'cases': 800, 'match': 121, 'protocol-refusal': 655,
                                                        'decoder-refusal': 24, 'mismatch': 0})
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
        self.assertEqual(list(result['mutations']), ['mutations-d1-d6', 'mutations-d8ab', 'mutations-d8c', 'mutations-fixes2', 'mutations-payload'])
        payload = result['mutations']['mutations-payload']
        self.assertEqual(payload['total'], 328)
        self.assertEqual(payload['outcomes'], {'error': 22, 'killed-by-vector': 20, 'rejected-by-build': 282, 'survived': 4})
        self.assertEqual(payload['survivors'], ['finish-json-hexdigit', 'last-spec-tests-conds', 'payload-whole-as-number', 'record-num-hidden'])
        self.assertEqual(payload['runtime'], {'vector': 139, 'generator': 112, 'checks': 12, 'unbuilt': 7, 'none': 12})
        self.assertEqual(len(payload['proofsAlone']), 12)
        current = result['mutations']['mutations-d8c']
        self.assertEqual(current['total'], 291)
        self.assertEqual(current['outcomes'], {'error': 22, 'killed-by-vector': 24, 'rejected-by-build': 243, 'survived': 2})
        self.assertEqual(current['survivors'], ['finish-json-hexdigit', 'last-spec-tests-conds'])
        self.assertEqual(current['guards']['d8-of-not-root-skipped'], 'killed-by-vector')
        self.assertEqual(current['guards']['d8-unrelated-ignores-target-of'], 'killed-by-vector')
        before = result['mutations']['mutations-d8ab']
        self.assertEqual(before['total'], 210)
        self.assertEqual(before['outcomes'], {'error': 15, 'killed-by-vector': 28, 'rejected-by-build': 165, 'survived': 2})
        self.assertEqual(before['survivors'], ['finish-json-hexdigit', 'last-spec-tests-conds'])
        self.assertEqual(before['guards']['d8-of-not-root-skipped'], 'killed-by-vector')
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
