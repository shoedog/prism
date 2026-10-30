"""Preregistered resident pairing and bounded collection budgets."""
import json
import math
from pathlib import Path

SOURCE = ('arm','instance','cycle','attempt','generation')
PINS = ('corpus_pin','workload_hash','commit','features','binary_sha256','mode','pair_id')


def witness(row):
    response, disposal = row.get('last_response_epoch_ns'), row.get('disposal_end_epoch_ns')
    if type(response) is not int or type(disposal) is not int or disposal < response:
        raise ValueError('resident pairing requires ordered integer response/disposal epochs')
    if any(row.get(k) is None for k in SOURCE):
        raise ValueError('resident pairing missing source identity')
    return dict({k:row[k] for k in SOURCE},delta_ns=disposal-response,
                last_response_epoch_ns=response,disposal_end_epoch_ns=disposal)


def prepare(meta):
    if not isinstance(meta.get('first_request'),dict):
        raise ValueError('first_request must be a request object')
    default = meta['requested_idle_secs']*meta['cycles']+180
    if meta['arm']=='resident':
        path = meta.get('resident_pair')
        if not path:
            raise ValueError('resident_pair is required')
        paired = json.loads(Path(path).read_text())
        selected = [x for x in paired['rows'] if x.get('score_memory') is True and x.get('arm') in ('A','A+C')]
        if len(selected) != 1:
            raise ValueError('resident pairing requires one unique scored candidate')
        match = selected[0]
        if any(meta.get(k) is None or meta.get(k) != match.get(k) for k in PINS):
            raise ValueError('resident pairing provenance mismatch')
        meta['resident_pairing'] = witness(match)
        if meta['cycles'] != 1:
            raise ValueError('resident pairing requires one local cycle')
        default = meta['resident_pairing']['delta_ns']/1000000000+30+180
    timeout = meta.get('timeout_secs',default)
    if isinstance(timeout,bool) or not isinstance(timeout,(int,float)) or not math.isfinite(timeout) or timeout <= 0:
        raise ValueError('timeout_secs must be positive and finite')
    if meta['arm']=='resident' and timeout < default:
        raise ValueError('resident timeout_secs too short; requires delta +30s +180s launch/query budget')
    return timeout


def validate(base, candidate):
    if base.get('arm') == 'resident' and base.get('resident_pairing') != witness(candidate):
        raise ValueError('resident pairing source mismatch')
