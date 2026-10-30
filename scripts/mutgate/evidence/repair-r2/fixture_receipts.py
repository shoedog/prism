"""Capture matched, dependency-free HEAD/candidate controls without git writes."""
import importlib.util
from importlib.machinery import SourceFileLoader
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / 'scripts/mutgate'))
import mutgate
import test_mutgate

spec = importlib.util.spec_from_loader('round1', SourceFileLoader('round1', str(HERE / 'base-mutgate.py.txt')))
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
fixtures = {
    'W1-AFTER-minus-f': (
        'pub fn f() -> u32 {\n    let ignored = 1;\n    line!()\n}\n'
        'const AFTER: u32 = line!();\n#[test]\nfn t() { assert_eq!(AFTER - f(), 2); }\n',
        'let ignored = 1;', 'let ignored = 2;'),
    'W1-hidden-track-caller': (
        'pub fn f() -> u32 {\n    let ignored = 1;\n    location()\n}\n'
        'const AFTER: u32 = line!();\n#[track_caller]\n'
        'fn location() -> u32 { std::panic::Location::caller().line() }\n'
        '#[test]\nfn t() { assert_eq!(AFTER - f(), 2); }\n',
        'let ignored = 1;', 'let ignored = 2;'),
    'unwrap-value-kill': (
        'fn f() -> u32 { Some(1).unwrap() }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n',
        'Some(1)', 'Some(2)'),
    'unwrap-panic-kill': (
        'fn f() -> u32 { Some(1).unwrap() }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n',
        'Some(1)', 'None::<u32>'),
    'local-safe-macro-kill': (
        'macro_rules! lit { () => { 1 }; }\nfn f() -> u32 { lit!() + 1 }\n'
        '#[test]\nfn t() { assert_eq!(f(), 2); }\n',
        'lit!() + 1', 'lit!() + 2'),
    'payload-location-confirmation': (
        'fn f() -> bool { true }\n#[test]\nfn t() { assert!(f(), "observed src/lib.rs:12:3"); }\n',
        '{ true }', '{ false }'),
    'should-panic-location-confirmation': (
        'fn f() -> bool { true }\n#[test]\n#[should_panic(expected="src/lib.rs:")]\n'
        'fn t() { if f() { panic!("src/lib.rs:"); } else { panic!("wrong"); } }\n',
        '{ true }', '{ false }'),
}
records = {}
for label, module in (('prechange-HEAD', base), ('candidate', mutgate)):
    test_mutgate.mutgate = module
    records[label] = {}
    for name, (source, original, replacement) in fixtures.items():
        case = test_mutgate.GateRegressionTests()
        case.setUp()
        try:
            status, summary = case.gate(source, {'m': ['src/lib.rs', original, replacement, 't']}, 'schema')
            records[label][name] = {'exit': status, 'source': source, 'original': original,
                                    'replacement': replacement, 'summary': summary}
        finally:
            case.doCleanups()
test_mutgate.mutgate = mutgate
(HERE / 'fixture-receipts.json').write_text(json.dumps(records, indent=2) + '\n')
for name in fixtures:
    before = records['prechange-HEAD'][name]['summary']['results']['m']
    after_summary = records['candidate'][name]['summary']
    after = after_summary['results']['m']
    expected = 'SURVIVED' if name.startswith('W1-') else 'KILLED'
    assert before['verdict'] == after['verdict'] == expected, name
    if name in ('unwrap-value-kill', 'unwrap-panic-kill', 'local-safe-macro-kill'):
        assert before['mode'] == 'text' and after['mode'] == 'schema', name
        assert not after_summary['static'] and after_summary['text_confirmations'] == 0, name
    if name == 'W1-hidden-track-caller':
        assert after['schema_observation']['killed'] and after_summary['text_confirmations'] == 1
    if name.endswith('confirmation'):
        assert after['mode'] == 'schema+text' and after_summary['text_confirmations'] == 1
    print(name, expected, before['mode'], '->', after['mode'])
