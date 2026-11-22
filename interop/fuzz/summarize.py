#!/usr/bin/env python3
"""Import small evidence artifacts and derive reports without raw files."""
import argparse
import collections
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
CAMPAIGNS = ROOT / 'interop/fuzz/evidence'
MUTATIONS = ROOT / 'model/mutate/evidence'
REFERENCE = CAMPAIGNS / 'step1-3/d1-mismatches.json'
CATEGORIES = ('match', 'protocol-refusal', 'decoder-refusal', 'mismatch')
STATUSES = {'rejected-by-build', 'killed-by-vector', 'killed-by-run-check', 'killed-by-checks', 'error', 'survived'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(path):
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f'{path}: {exc}') from exc


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def label_path(base, label):
    require(bool(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]*', label)) and label not in ('.', '..'), 'invalid label')
    return base / label


def totals(summary):
    rows = summary['counts']
    require(bool(rows), 'empty campaign counts')
    result: dict[str, int] = dict.fromkeys(('cases', *CATEGORIES), 0)
    for family, row in rows.items():
        for key in result:
            require(type(row[key]) is int and row[key] >= 0, f'{family}: invalid {key}')
            result[key] += row[key]
        require(row['cases'] == sum(row[k] for k in CATEGORIES), f'{family}: category sum differs')
    for key, field in [('mismatch', 'mismatches'), ('decoder-refusal', 'decoderRefusals'), ('protocol-refusal', 'protocolRefusals')]:
        require(summary[field] == result[key], f'{field}: total differs')
    require(summary['requested'] == result['cases'], 'requested total differs')
    return result


def example(finding):
    answer = finding['answer']
    require(answer['status'] == 'mismatch', 'answer is not a mismatch')
    require(answer['lean'] == answer['rust'] != answer['js'], 'wrong answer partition')
    return {'layer': finding['vector']['layer'], 'input': finding['vector']['input'],
            'answers': {k: answer[k] for k in ('js', 'lean', 'rust')}}


def validate_cases(summary, data, reference):
    result = totals(summary)
    require(len(data['cases']) == result['mismatch'], 'mismatch count differs')
    require(data['examples'] == reference, f'D1 examples differ from {REFERENCE.parent.name}')
    by_family = collections.Counter()
    seen = set()
    for case in data['cases']:
        require(case['seed'] == summary['seed'], 'case seed differs')
        require(case['family'] in summary['counts'], 'unknown case family')
        require(type(case['index']) is int and case['index'] >= 0, 'invalid case index')
        identity = (case['seed'], case['index'])
        require(identity not in seen, 'duplicate case index')
        seen.add(identity)
        require(case['raw'] == 'raw' and case['reduced'] == 'reduced', 'invalid example reference')
        shrink = case['shrink']
        require(all(type(shrink[k]) is int and shrink[k] >= 0 for k in ('trials', 'rounds')) and type(shrink['bounded']) is bool, 'invalid shrink stats')
        by_family[case['family']] += 1
    for family, row in summary['counts'].items():
        require(by_family[family] == row['mismatch'], f'{family}: mismatch count differs')
    require(sum(c['shrink']['trials'] for c in data['cases']) == summary['shrinkTrials'], 'shrink trial total differs')
    return result


def import_campaign(raw, label):
    summary = read(raw / 'summary.json')
    totals(summary)
    paths = sorted((raw / 'mismatches').glob('*.json'))
    require(len(paths) == summary['mismatches'], 'missing mismatch files')
    if REFERENCE.exists():
        reference = read(REFERENCE)['examples']
    else:
        require(label == REFERENCE.parent.name and paths, f'import {REFERENCE.parent.name} first')
        first = read(paths[0])
        reference = {'raw': example(first), 'reduced': example(first['shrink'])}
        require(reference['raw']['layer'] == reference['reduced']['layer'] == 'lens/section', 'reference layer differs')
    cases = []
    for path in paths:
        finding = read(path)
        require(example(finding) == reference['raw'] and example(finding['shrink']) == reference['reduced'], f'{path}: not the known D1 example')
        fuzz = finding['vector']['fuzz']
        require(finding['shrink']['vector']['fuzz'] == fuzz, f'{path}: reduced case metadata differs')
        cases.append({**{k: fuzz[k] for k in ('seed', 'index', 'family')}, 'raw': 'raw', 'reduced': 'reduced',
                      'shrink': {k: finding['shrink'][k] for k in ('trials', 'rounds', 'bounded')}})
    data = {'examples': reference, 'cases': cases}
    validate_cases(summary, data, reference)
    target = label_path(CAMPAIGNS, label)
    write(target / 'summary.json', summary)
    write(target / 'd1-mismatches.json', data)


def command_status(command):
    require(command['status'] in ('ok', 'nonzero'), 'command infrastructure failure')
    require(not command.get('signal') and not command.get('error'), 'command signal or error')
    require(type(command.get('code')) is int, 'missing exit code')
    require((command['code'] == 0) == (command['status'] == 'ok'), 'inconsistent exit code')
    return {'status': command['status'], 'code': command['code'], 'signal': command.get('signal'), 'error': command.get('error')}


def import_mutation(path, label):
    full = read(path)
    # merge.mjs retains the catalogue size and verdicts, but omits shard baselines.
    merged = 'catalogue' in full
    if merged:
        artifact = {'catalogue': full['catalogue']}
    else:
        baseline = {key: command_status(full['baseline'][key]) for key in ('build', 'gen')}
        baseline['changedPaths'] = full['baseline']['changedPaths']
        artifact = {'baseline': baseline, 'finalBaseline': command_status(full['finalBaseline'])}
    mutants = []
    for mutant in full['mutants']:
        require(mutant['status'] in STATUSES, 'unknown mutant status')
        commands = [command_status(c) for c in mutant['commands']]
        require(bool(commands), 'mutant has no commands')
        item = {'id': mutant['id'], 'status': mutant['status'], 'commands': commands}
        if 'runtime' in mutant:
            item['runtimeVerdict'] = mutant['runtime']['verdict']
        if mutant['status'] == 'error':
            cmd = mutant['commands'][-1]
            require(cmd['command'][:3] == ['lake', 'exe', 'gen'] and commands[-1]['status'] == 'nonzero', 'error is not a generator failure')
            log = pathlib.Path(cmd['log'])
            if not log.is_absolute():
                log = ROOT / log
            if merged and not log.is_file():
                require(mutant['failureDetail'] == 'Generator nonzero', 'missing generator failure detail')
                item['failureDetail'] = mutant['failureDetail']
            else:
                lines = [line.strip() for line in log.read_text().splitlines() if 'uncaught exception:' in line]
                require(len(lines) == 1, 'generator assertion is missing or ambiguous')
                item['assertion'] = lines[0]
        mutants.append(item)
    artifact['mutants'] = mutants
    derived = mutation_report(artifact)
    if merged:
        require(derived['outcomes'] == full['counts'], 'merged outcome counts differ')
        require(derived.get('runtime') == full.get('runtime'), 'merged runtime counts differ')
        require(derived.get('proofsAlone') == full.get('proofsAlone'), 'merged proofs-alone entries differ')
    write(label_path(MUTATIONS, label) / 'summary.json', artifact)


def mutation_report(data):
    if 'catalogue' in data:
        require(len(data['mutants']) == data['catalogue'], 'incomplete merged catalogue')
    else:
        require(data['baseline']['changedPaths'] == [], 'baseline changed paths')
        for command in (data['baseline']['build'], data['baseline']['gen'], data['finalBaseline']):
            require(command_status(command)['status'] == 'ok', 'baseline failed')
    outcomes = collections.Counter()
    runtime, proofs_alone = collections.Counter(), []
    guards, survivors, errors, ids = {}, [], [], set()
    for mutant in data['mutants']:
        ident, status = mutant['id'], mutant['status']
        require(isinstance(ident, str) and ident and ident not in ids, 'duplicate or invalid mutant id')
        ids.add(ident)
        require(status in STATUSES and mutant['commands'], 'invalid mutant outcome')
        for command in mutant['commands']:
            command_status(command)
        outcomes[status] += 1
        if 'runtimeVerdict' in mutant:
            verdict = mutant['runtimeVerdict']
            require(status == 'rejected-by-build' and verdict in ('unbuilt', 'generator', 'vector', 'run-check', 'checks', 'none', 'unknown'), 'invalid second verdict')
            runtime[verdict] += 1
            if verdict == 'none':
                proofs_alone.append(ident)
        if ident.startswith(('step1-', 'd1-', 'd2-', 'd8-')):
            guards[ident] = status
        if status == 'survived':
            survivors.append(ident)
        if status == 'error':
            if 'catalogue' in data and 'failureDetail' in mutant:
                require(mutant['failureDetail'] == 'Generator nonzero' and mutant['commands'][-1]['status'] == 'nonzero', 'invalid generator failure')
                errors.append({'id': ident, 'failureDetail': mutant['failureDetail']})
            else:
                require(mutant.get('assertion', '').startswith('uncaught exception:'), 'missing assertion')
                errors.append({'id': ident, 'assertion': mutant['assertion']})
    require(bool(ids), 'empty mutation run')
    result = {'total': len(ids), 'outcomes': dict(sorted(outcomes.items())), 'guards': guards, 'survivors': sorted(survivors), 'errors': errors}
    if runtime:
        result.update(runtime=dict(runtime), proofsAlone=proofs_alone)
    return result


def report():
    reference = read(REFERENCE)['examples']
    campaigns = {}
    for path in sorted(CAMPAIGNS.glob('*/summary.json')):
        summary = read(path)
        data = read(path.parent / 'd1-mismatches.json')
        counts = validate_cases(summary, data, reference)
        campaigns[path.parent.name] = {'totals': counts, 'families': summary['counts'],
            'shrinkTrials': sum(c['shrink']['trials'] for c in data['cases']),
            'shrinkRounds': sum(c['shrink']['rounds'] for c in data['cases']),
            'shrinkBounded': sum(c['shrink']['bounded'] for c in data['cases'])}
    mutations = {path.parent.name: mutation_report(read(path)) for path in sorted(MUTATIONS.glob('*/summary.json'))}
    return {'campaigns': campaigns, 'mutations': mutations}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    campaign = sub.add_parser('campaign', help='import a campaign raw directory')
    campaign.add_argument('rawdir', type=pathlib.Path)
    campaign.add_argument('label')
    mutation = sub.add_parser('mutation', help='import a runner result or merged shard JSON file')
    mutation.add_argument('fulljson', type=pathlib.Path)
    mutation.add_argument('label')
    sub.add_parser('report', help='derive counts from evidence artifacts only')
    args = parser.parse_args()
    if args.command == 'campaign':
        import_campaign(args.rawdir if args.rawdir.is_absolute() else ROOT / args.rawdir, args.label)
    elif args.command == 'mutation':
        import_mutation(args.fulljson if args.fulljson.is_absolute() else ROOT / args.fulljson, args.label)
    else:
        print(json.dumps(report(), indent=2))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, OSError, TypeError) as exc:
        print(f'evidence error: {exc}', file=sys.stderr)
        sys.exit(1)
