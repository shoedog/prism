"""Bounded O-lite file collector. Production timestamps, never tail arrival, are event time."""
import os
import re
import stat
import time
from pathlib import Path

COMMON = set(('identity pid instance_start epoch_ns mono_ns effective_relief event reason attempt '
              'generation configured_idle_secs observed_idle_secs duration_ns cpg load_parse_ns '
              'f_validation_ns index_ns bootstrap_ns sidecar sequence released_bytes mono_ms outcome').split())
PAIR = set(('sample_before_ns sample_after_ns sample_before_epoch_ns sample_after_epoch_ns '
            'heap_before heap_after footprint_before footprint_after').split())
INTEGERS = set(('pid instance_start epoch_ns mono_ns attempt generation sequence released_bytes '
                'mono_ms timestamp_ms duration_ms').split())
OPTIONAL = set(('duration_ns load_parse_ns f_validation_ns index_ns bootstrap_ns').split()) | PAIR
MARKERS = {'unknown', 'not_applicable'}


def identity(event):
    return tuple(str(event[k]) for k in ('pid', 'instance_start', 'identity'))


def parse_line(raw):
    """Return (event, exclusion); unrelated diagnostic lines are not events."""
    prefix = next((p for p in (b'prism-mcp daemon: ', b'prism-mcp: ') if raw.startswith(p)), None)
    if prefix is None:
        return None, None
    body = raw[len(prefix):]
    if b'the log reached its cap' in body:
        return None, 'log_cap'
    if not any(token in body for token in (b'identity=', b'event=', b'sequence=')):
        return None, None
    if not raw.endswith(b'\n'):
        return None, 'partial_line'
    body = body[:-1]
    metadata = body.split(b' outcome=', 1)[0]
    sequence = re.search(rb' sequence=[0-9]+(?= released_bytes=)', metadata)
    uncapped = body if sequence is None else body[:sequence.start()] + body[sequence.end():]
    if len(uncapped) >= 967:
        return None, 'possibly_capped'
    try:
        text = body.decode('utf-8', 'strict')
        head, outcome = text.split(' outcome=', 1)
        tokens = [token.split('=', 1) for token in head.split(' ')]
        if any(len(t) != 2 or not t[1] for t in tokens):
            raise ValueError('invalid token')
        event = dict(tokens, outcome=outcome)
        if len(event) != len(tokens) + 1 or not COMMON <= event.keys() or not outcome:
            raise ValueError('missing or duplicate field')
        required = COMMON | (PAIR if event['event'] == 'relief' else set())
        if not required <= event.keys() or event['effective_relief'] not in ('true', 'false'):
            raise ValueError('missing pair or boolean')
        for key in INTEGERS | OPTIONAL:
            if key not in event:
                continue
            value = event[key]
            if key in OPTIONAL and value in MARKERS:
                continue
            if not value.isascii() or not value.isdecimal():
                raise ValueError('invalid unsigned integer')
            event[key] = int(value)
        for key in ('configured_idle_secs', 'observed_idle_secs'):
            value = event[key]
            if value not in MARKERS:
                if not re.fullmatch(r'[0-9]+(?:\.[0-9]+)?', value):
                    raise ValueError('invalid seconds')
                event[key] = float(value)
        if event['pid'] == 0 or event['sequence'] == 0:
            raise ValueError('zero identity/sequence')
        event['effective_relief'] = event['effective_relief'] == 'true'
        return event, None
    except (ValueError, UnicodeError):
        return None, 'malformed_event'


class Collector:
    """Tail all required sinks; sticky exclusions preserve incomplete raw intervals.

    A file observed through two hardlinks is read once. Two lines with the same
    process sequence at different offsets are a duplicate, not silent deduplication.
    """
    def __init__(self, streams):
        self.streams = streams
        self.events, self.reasons, self.raw = [], [], []
        self.states, self.sequences, self.locations = {}, {}, set()

    def exclude(self, reason):
        if reason not in self.reasons:
            self.reasons.append(reason)

    def poll(self):
        for stream in self.streams:
            path = str(stream['path'])
            state = self.states.setdefault(path, {'offset': 0, 'pending': b'', 'file_id': None,
                                                  'events': 0})
            try:
                with open(path, 'rb') as source:
                    info = os.fstat(source.fileno())
                    if not stat.S_ISREG(info.st_mode):
                        raise OSError('sink is not regular')
                    file_id = (info.st_dev, info.st_ino)
                    if state['file_id'] not in (None, file_id):
                        self.exclude('replaced_stream')
                    if info.st_size < state['offset']:
                        self.exclude('truncated_stream')
                    if state.get('head'):
                        if source.read(len(state['head'])) != state['head']:
                            self.exclude('truncated_stream')
                        source.seek(max(0,state['offset']-128))
                        if source.read(min(128,state['offset'])) != state['edge']:
                            self.exclude('truncated_stream')
                    state['file_id'] = file_id
                    source.seek(state['offset'])
                    data = state['pending'] + source.read()
                    offset = state['offset'] - len(state['pending'])
                    state['offset'] = source.tell()
                    source.seek(0)
                    state['head'] = source.read(min(256,state['offset']))
                    source.seek(max(0,state['offset']-128))
                    state['edge'] = source.read(min(128,state['offset']))
            except OSError:
                self.exclude('missing_stream')
                continue
            lines = data.splitlines(keepends=True)
            state['pending'] = b''
            for raw in lines:
                if not raw.endswith(b'\n'):
                    state['pending'] = raw
                    break
                location = (file_id, offset)
                offset += len(raw)
                if location in self.locations:
                    continue
                self.locations.add(location)
                event, reason = parse_line(raw)
                self.raw.append({'sink': path, 'file_id': file_id, 'offset': location[1],
                                 'raw': raw.decode('utf-8', 'replace'), 'reason': reason})
                if reason:
                    self.exclude(reason)
                if event is None:
                    continue
                state['events'] += 1
                if stream.get('instance') and identity(event) != identity(stream['instance']):
                    self.exclude('wrong_instance')
                if stream.get('pid') is not None and event['pid'] != stream['pid']:
                    self.exclude('wrong_instance')
                key = identity(event)
                previous = self.sequences.get(key, 0)
                sequence = event['sequence']
                if sequence <= previous:
                    self.exclude('duplicate_sequence')
                elif sequence != previous + 1:
                    self.exclude('sequence_gap')
                self.sequences[key] = sequence
                event.update(sink=path, file_id=file_id, offset=location[1],
                             received_epoch_ns=time.time_ns())
                self.events.append(event)

    def finish(self):
        for stream in self.streams:
            state = self.states.get(str(stream['path']), {})
            if state.get('pending'):
                self.exclude('partial_line')
            if not state.get('events') and stream.get('require_events', True):
                # An alias of a read sink is complete, even though it contributes no second copy.
                aliased = any(s is not state and s.get('file_id') == state.get('file_id')
                              and s.get('events') for s in self.states.values())
                if not aliased:
                    self.exclude('missing_stream')
