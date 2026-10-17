#!/usr/bin/env python3
"""A stand-in for prism-mcp in the fan-out driver's tests (test_fanout.py). It speaks just enough
MCP: `initialize`, notifications, `tools/call` (refresh_index included). Environment:
FAKE_WARMING=N answers the first N tool calls retryable (as an index that is warming);
FAKE_DIE_AFTER=N exits after N tool calls; FAKE_DELAY_MS=N waits before each answer;
FAKE_DAEMON_PID=P makes a `--shared` process report daemon pid P on standard error;
`daemon-shutdown` as the first argument prints a line and exits FAKE_SHUTDOWN_EXIT (default 0).
It writes the PRISM_MCP_CONCISE_SHAPE it was given to standard error, and at the end of its input
`fake: N tool calls`. FAKE_REFRESH=retryable|rpc_error|tool_error answers refresh_index so. A
`--shared` process appends a line to `fake.log` in $PRISM_MCP_RUNTIME_DIR/prism-mcpd, as a daemon
would to its log."""
import json
import os
import sys
import time


def answer(ident, result):
    sys.stdout.write(json.dumps({'jsonrpc': '2.0', 'id': ident, 'result': result}) + '\n')
    sys.stdout.flush()


def main():
    if sys.argv[1:2] == ['daemon-shutdown']:
        print('fake: nothing to shut down')
        sys.exit(int(os.environ.get('FAKE_SHUTDOWN_EXIT', '0')))
    sys.stderr.write('fake: PRISM_MCP_CONCISE_SHAPE=%s\n' % os.environ.get('PRISM_MCP_CONCISE_SHAPE', '<unset>'))
    if '--shared' in sys.argv and os.environ.get('PRISM_MCP_RUNTIME_DIR'):
        logs = os.path.join(os.environ['PRISM_MCP_RUNTIME_DIR'], 'prism-mcpd')
        os.makedirs(logs, exist_ok=True)
        with open(os.path.join(logs, 'fake.log'), 'a') as log:
            log.write('fake daemon log\n')
    if '--shared' in sys.argv:
        sys.stderr.write('prism-mcp: shared daemon pid %s (started 7, generation 1) at /fake.sock\n'
                         % os.environ.get('FAKE_DAEMON_PID', '1'))
        sys.stderr.flush()
    warming = int(os.environ.get('FAKE_WARMING', '0'))
    die_after = int(os.environ.get('FAKE_DIE_AFTER', '0'))
    delay = int(os.environ.get('FAKE_DELAY_MS', '0')) / 1000.0
    calls = 0
    for line in sys.stdin:
        message = json.loads(line)
        if 'id' not in message:
            continue
        if message['method'] == 'initialize':
            answer(message['id'], {'serverInfo': {'name': 'prism-mcp', 'version': 'fake'}, 'capabilities': {}})
            continue
        calls += 1
        if die_after and calls > die_after:
            sys.exit(1)
        time.sleep(delay)
        if calls <= warming:
            answer(message['id'], {'isError': True, 'content': [{'type': 'text', 'text': 'warming'}],
                                   '_meta': {'prism/retryable': True, 'prism/index_state': 'warming'}})
        elif message['params']['name'] == 'refresh_index':
            refresh = os.environ.get('FAKE_REFRESH', 'ok')
            if refresh == 'rpc_error':
                sys.stdout.write(json.dumps({'jsonrpc': '2.0', 'id': message['id'],
                                             'error': {'code': -32603, 'message': 'fake'}}) + '\n')
                sys.stdout.flush()
            elif refresh == 'retryable':
                answer(message['id'], {'isError': True, 'content': [], '_meta': {'prism/retryable': True}})
            else:
                answer(message['id'], {'isError': refresh == 'tool_error', 'content': [{'type': 'text', 'text': '{}'}],
                                       '_meta': {'prism/refresh_status': 'unchanged'}})
        else:
            answer(message['id'], {'isError': False, 'content': [{'type': 'text', 'text': '{}'}]})
    sys.stderr.write('fake: %d tool calls\n' % calls)


if __name__ == '__main__':
    main()
