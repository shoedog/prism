"""One-shot event deadlines. No periodic memory series, catch-up pass or interpolation."""
from memreport_idle_clock import Clocks, Continuity
from memreport_idle_sampling import boundary_reason, deadline, sweep


class Engine:
    def __init__(self, sampler, members, inventory, clocks=None):
        self.sampler, self.members, self.inventory = sampler, members, inventory
        self.clocks = clocks if clocks is not None else Clocks()
        self.guard = Continuity()
        self.samples, self.pending, self.observations, self.events = [], [], [], []
        self.reasons = []
        self.completed_queries = {}

    def clock(self):
        value = self.clocks.valid_read()
        reason = self.guard.add(value)
        self.observations.append(dict(clock=value, reason=reason))
        if reason in ('clock_step','sleep_or_unknown_continuity'):
            self.reasons.append(reason)
        return value

    def enqueue(self, boundary, due, event):
        now = self.clock()
        mono_due = None if now is None else now['mono_ns'] + due - now['epoch_ns']
        self.pending.append(dict(boundary=boundary, due_epoch_ns=due, due_mono_ns=mono_due,
                                 attempt=event.get('attempt'), generation=event.get('generation'),
                                 anchor_epoch_ns=event['epoch_ns'], join_reason='missing_clock' if now is None else None))

    def observe(self, events):
        for event in events:
            self.events.append(event)
            if event['outcome'] != 'complete':
                continue
            if event['event'] == 'publication':
                self.enqueue('loaded', deadline(event, 0), event)
            elif event['event'] == 'disposal_end':
                self.enqueue('idle5', deadline(event, 5), event)
                self.enqueue('idle30', deadline(event, 30), event)

    def served(self, attempts, arrival):
        for attempt in attempts:
            self.completed_queries[attempt] = arrival

    def take(self, task):
        before = self.clock()
        reason = 'missing_clock' if before is None else boundary_reason(
            task['due_epoch_ns'], before['epoch_ns'], before['epoch_ns'])
        if reason:
            result = dict(start_epoch_ns=None if before is None else before['epoch_ns'],
                          end_epoch_ns=None if before is None else before['epoch_ns'],
                          footprint_bytes=None, rss_bytes=None, members=[], reasons=[reason])
        else:
            result = sweep(self.sampler, self.members, self.inventory, self.clock)
        after = self.clock()
        if before is not None and after is not None:
            result.update(start_epoch_ns=before['epoch_ns'], end_epoch_ns=after['epoch_ns'])
            reason = boundary_reason(task['due_epoch_ns'], before['epoch_ns'], after['epoch_ns'])
            if reason:
                result['reasons'].append(reason)
            result['lateness_ns'] = after['epoch_ns'] - task['due_epoch_ns']
        else:
            result['reasons'].append('missing_clock')
        result['reasons'].extend(self.reasons)
        if task.get('join_reason'):
            result['reasons'].append(task['join_reason'])
        if result['reasons']:
            result['footprint_bytes'] = result['rss_bytes'] = None
        result.update(task, clock_before=before, clock_after=after,
                      cumulative_drift_ns=self.guard.cumulative_drift_ns)
        self.samples.append(result)
        return result

    def tick(self):
        now = self.clock()
        if now is None:
            return
        for task in list(self.pending):
            if task['boundary'] == 'loaded' and task['attempt'] not in self.completed_queries:
                continue
            if task['due_mono_ns'] is None or now['mono_ns'] >= task['due_mono_ns']:
                self.take(task)
                self.pending.remove(task)

    def pre_index(self):
        now = self.clock()
        due = now['epoch_ns'] if now else 0
        return self.take(dict(boundary='pre_index', due_epoch_ns=due, attempt=None,
                              generation=None, anchor_epoch_ns=due))

    def finish(self, events):
        for sample in self.samples:
            if sample['boundary'] not in ('loaded','idle5','idle30') or sample['end_epoch_ns'] is None:
                continue
            crossed = any(e['event'] in ('build_start','refresh','auto_refresh') and
                          sample['anchor_epoch_ns'] < e['epoch_ns'] <= sample['end_epoch_ns']
                          for e in events)
            failed = any(e['event'].startswith('disposal') and e['outcome'] != 'complete' and
                         e['attempt'] == sample['attempt'] for e in events)
            if crossed or failed:
                sample['reasons'].append('crossed_lifecycle')
                sample['footprint_bytes'] = sample['rss_bytes'] = None
