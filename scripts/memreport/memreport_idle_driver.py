"""Bounded idle CLI, deliberately independent of the eager memory-report driver."""
import hashlib
import json
import sys
import time
from pathlib import Path

from memreport_idle import Collector, identity
from memreport_idle_process import Process
from memreport_idle_readout import admit, readout, rows_for_run
from memreport_idle_sampling import Sampler, resident_deadline
from memreport_idle_schedule import Engine
from memreport_idle_pairing import prepare
from memreport_idle_recovery import recover
from memreport_idle_rows import relief_settings

PREREQUISITES = ('soak_complete','c0_pins','quiet_serial','swap_non_increasing',
                 'reserved_window','all_prerequisites_landed')


def command(meta):
    args = [meta['binary'], '--repo', meta['repo'], '--cache-dir', meta['cache_dir'],
            '--first-call-wait', str(meta.get('first_call_wait_secs',20)),
            '--index-idle-secs', str(meta['requested_idle_secs'])]
    if meta.get('requested_relief'):
        args += ['--index-idle-pressure-relief']
    if meta.get('startup','lazy') == 'background':
        args += ['--warm-at-startup']
    if meta['mode'] == 'shared':
        if not meta.get('runtime_dir'):
            raise ValueError('shared requires private runtime_dir')
        args += ['--shared']
    return args


def validate_manifest(meta):
    if meta.get('mode')=='shared' and meta.get('runtime_dir'):
        runtime = Path(meta['runtime_dir']).resolve()
        root = Path(__file__).resolve().parents[2]
        if not Path(meta['runtime_dir']).is_absolute() or runtime == root or root in runtime.parents:
            raise ValueError('runtime logs must be absolute and outside checkout')
    if meta.get('startup','lazy') not in ('lazy','background'):
        raise ValueError('idle collector supports Lazy/Background only')
    if any(type(meta.get(k,1)) is not int or meta.get(k,1) < 1 for k in ('cycles','max_polls')):
        raise ValueError('cycles and max_polls must be positive integers')
    if meta.get('mode') not in ('stdio','shared') or meta.get('arm') not in ('resident','A','A+C','reference'):
        raise ValueError('invalid mode/arm')
    idle = meta.get('requested_idle_secs')
    if type(idle) is not int or idle < 0:
        raise ValueError('idle seconds must be a nonnegative integer')
    if not meta.get('smoke_only') and meta['arm'] != 'resident' and idle != 600:
        raise ValueError('admitted enabled runs require 600 seconds')
    if meta['arm'] == 'resident' and idle != 0:
        raise ValueError('resident must disable idle')
    if type(meta.get('requested_relief')) is not bool:
        raise ValueError('requested_relief must be boolean')
    if meta['arm'] == 'A' and meta['requested_relief']:
        raise ValueError('A is the no-op arm')
    if meta['arm'] == 'A+C' and not meta['requested_relief']:
        raise ValueError('A+C requires relief')
    reasons = ['smoke_only'] if meta.get('smoke_only') else []
    prereq = meta.get('prerequisites',{})
    if any(prereq.get(k) is not True for k in PREREQUISITES) or prereq.get('free_bytes',0) < 50*1024**3:
        reasons.append('missing_prerequisite')
    return reasons


def external_output(path):
    path = Path(path).resolve()
    root = Path(__file__).resolve().parents[2]
    if path == root or root in path.parents:
        raise ValueError('raw output must be outside checkout')
    path.mkdir(parents=True,exist_ok=True)
    return path


def write(path, value):
    path.write_text(json.dumps(value,indent=2,sort_keys=True)+'\n')


def collect(meta, out):
    out = external_output(out)
    if (out/'run.json').exists():
        raise ValueError('run directory already used')
    state = dict(events=[],journal=[],samples=[],reasons=[])
    try:
        meta = dict(meta)
        meta['platform'] = sys.platform
        meta.update(relief_settings(meta))
        return collect_run(meta,out,state)
    except (Exception, KeyboardInterrupt) as error:
        meta = meta if isinstance(meta,dict) else {}
        result = recover(meta,state,error,time.time_ns())
        write(out/'run.json',result)
        return result


def error_text(error):
    return type(error).__name__+': '+str(error)


def collect_run(meta, out, state):
    reasons = validate_manifest(meta)
    state['reasons'] = reasons
    try:
        bound, pin_reasons = bind_binary(meta)
        meta.update(bound)
    except OSError as error:
        meta.update(binary_sha256=None)
        pin_reasons = ['binary_unavailable: '+str(error)]
    reasons.extend(pin_reasons)
    meta.setdefault('first_call_wait_secs',20)
    meta.setdefault('cycles',1 if meta['arm'] in ('resident','reference') else 2)
    budget = prepare(meta)
    write(out/'manifest.json',meta)
    sampler, journal, samples = Sampler(), state['journal'], state['samples']
    collector, engine, process = None, None, None
    active = {'demand_id':None, 'attempts':set()}
    started = time.monotonic()
    expires = started + budget
    def pump():
        if time.monotonic() >= expires:
            raise TimeoutError('run deadline')
        if collector:
            old = len(collector.events)
            collector.poll()
            new = collector.events[old:]
            for e in new:
                if e['event']=='build_start' and active['demand_id'] is not None:
                    active['attempts'].add(e['attempt'])
                    journal.append(dict(kind='attempt',demand_id=active['demand_id'],
                                        epoch_ns=e['epoch_ns'],attempt=e['attempt']))
            engine.observe(new)
            if active.get('served_arrival') is not None:
                engine.served(active['attempts'],active['served_arrival'])
            if active.get('unserved'):
                engine.unserved_queries.update(active['attempts'])
            engine.tick()
        return process.responses()
    def wait_for(predicate):
        while not predicate():
            pump(); time.sleep(.025)
    def request(method, params=None):
        wanted = process.send(method, params)
        while True:
            for response, arrival in pump():
                if response.get('id') == wanted:
                    return response, arrival
            if process.child.poll() is not None:
                raise EOFError('child exited before response')
            time.sleep(.025)
    try:
        launch_epoch_ns = time.time_ns()
        process = Process(command(meta),meta,out,sampler)
        request('initialize',dict(protocolVersion='2025-11-25',capabilities={},
                                 clientInfo=dict(name='memreport-idle',version='1')))
        process.send('notifications/initialized',notify=True)
        streams = process.discover()
        while streams is None:
            if time.monotonic() >= expires:
                raise TimeoutError('daemon discovery')
            time.sleep(.025); streams=process.discover()
        collector = Collector(streams)
        state['events'] = collector.events
        engine = Engine(sampler,process.members,process.inventory)
        state['samples'] = engine.samples
        engine.pre_index()
        collector.poll(); engine.observe(collector.events); engine.tick()
        if process.status:
            fields, status_reasons = observed_settings(process.status)
            meta.update(fields); reasons.extend(status_reasons)
            write(out/'status.json',process.status)
        else:
            meta['client_count'] = 1
        request_hash = hashlib.sha256(json.dumps(meta['first_request'],sort_keys=True,
                                                separators=(',',':')).encode()).hexdigest()
        for cycle in range(1,meta.get('cycles',2)+1):
            demand_id = str(cycle)
            demand = dict(kind='demand',demand_id=demand_id,epoch_ns=time.time_ns(),cycle=cycle,
                          request_hash=request_hash,prewarm=meta.get('prewarm',False))
            if cycle==1 and meta.get('startup')=='background':
                demand.update(epoch_ns=launch_epoch_ns,prewarm=True)
            journal.append(demand)
            active.update(demand_id=demand_id,attempts=set(),served_arrival=None,unserved=False)
            current_attempts = active['attempts']
            if cycle==1 and meta.get('startup')=='background':
                for e in collector.events:
                    if e['event']=='build_start':
                        current_attempts.add(e['attempt'])
                        journal.append(dict(kind='attempt',demand_id=demand_id,epoch_ns=e['epoch_ns'],attempt=e['attempt']))
            for poll in range(meta.get('max_polls',1)):
                if poll:
                    journal.append(dict(kind='poll',demand_id=demand_id,epoch_ns=time.time_ns()))
                response,arrival = request('tools/call',meta['first_request'])
                result = response.get('result',{})
                state = result.get('_meta',{}).get('prism/index_state')
                outcome = 'warming' if state=='warming' else 'failed' if ('error' in response or
                          result.get('isError')) else 'served'
                journal.append(dict(kind='response',demand_id=demand_id,epoch_ns=arrival,
                                    outcome=outcome,response=response))
                if outcome == 'served':
                    active['served_arrival'] = arrival
                    engine.served(current_attempts,arrival)
                pump()
                if outcome != 'warming':
                    break
                pause_end = time.monotonic()+meta.get('poll_interval_secs',.1)
                wait_for(lambda:time.monotonic() >= pause_end)
            if outcome != 'served':
                active['unserved'] = True
            if outcome == 'failed':
                journal.append(dict(kind='failed',demand_id=demand_id,epoch_ns=arrival,reason=str(response)))
                break
            if meta['arm']=='resident':
                match = meta['resident_pairing']
                local = [e for e in collector.events if e['event']=='publication' and
                         e['attempt'] in current_attempts and e['outcome']=='complete']
                if len(local) != 1:
                    raise ValueError('resident requires unique local publication')
                for delay,label in ((5,'idle5'),(30,'idle30')):
                    due = resident_deadline(arrival,match['last_response_epoch_ns'],match['disposal_end_epoch_ns'],delay)
                    engine.enqueue(label,due,dict(epoch_ns=arrival,attempt=local[0]['attempt'],
                                                  generation=local[0]['generation']))
            else:
                wait_for(lambda:any(e['event']=='disposal_end' and e['attempt'] in current_attempts for e in collector.events))
            hold = meta.get('smoke_hold_secs',30) if meta.get('smoke_only') else 30
            if hold >= 30:
                wait_for(lambda:not engine.pending)
        pump()
    except (Exception, KeyboardInterrupt) as error:
        journal.append(dict(kind='abandoned',demand_id=journal[-1].get('demand_id','none') if journal else 'none',
                            epoch_ns=time.time_ns(),reason=error_text(error)))
    finally:
        if process:
            try:
                process.close()
            except Exception as error:
                reasons.append('cleanup_failed: '+error_text(error))
                process.cleanup_confirmed = False
            if process.child.returncode != 0:
                journal.append(dict(kind='abandoned',demand_id=active['demand_id'],
                    epoch_ns=time.time_ns(),reason='child_exit_'+str(process.child.returncode)))
            if not process.cleanup_confirmed:
                reasons.append('shutdown_unconfirmed')
        if collector:
            collector.poll(); collector.finish(); reasons.extend(collector.reasons)
            write(out/'visibility.json',collector.raw)
        if engine:
            engine.finish(collector.events); samples=engine.samples; reasons.extend(engine.reasons)
            write(out/'clocks.json',engine.observations)
    events = collector.events if collector else []
    if events:
        index_pid = events[0]['pid']
        meta.update(instance=list(identity(events[0])),pid=index_pid,os_start=process.members.get(index_pid))
    rows = [admit(x) for x in rows_for_run(meta,events,journal,samples,reasons)]
    result = dict(manifest=meta,events=events,journal=journal,samples=samples,rows=rows,
                  reasons=sorted(set(reasons)),wall_secs=time.monotonic()-started)
    write(out/'run.json',result)
    return result


def add_parser(commands):
    idle = commands.add_parser('idle',help='normal-serving idle lifecycle collector (not eager memreport)')
    sub = idle.add_subparsers(dest='idle_command',required=True)
    run = sub.add_parser('collect'); run.add_argument('--manifest',type=Path,required=True)
    run.add_argument('--out',type=Path,required=True)
    run.set_defaults(handler=lambda a:collect(json.loads(a.manifest.read_text()),a.out))
    cross = sub.add_parser('cross-check'); cross.add_argument('--pid',type=int,required=True)
    cross.add_argument('--out',type=Path,required=True)
    def cross_check(args):
        from memreport_idle_sampling import cross_check
        cross_check(args.pid,external_output(args.out))
    cross.set_defaults(handler=cross_check)
    score = sub.add_parser('readout'); score.add_argument('--run',type=Path,action='append',required=True)
    score.add_argument('--out',type=Path,required=True)
    def summarize(args):
        output = external_output(args.out.parent)/args.out.name
        rows = []
        for path in args.run:
            run = json.loads(path.read_text())
            rows.extend(rows_for_run(run['manifest'],run['events'],run['journal'],
                                     run['samples'],run['reasons']))
        write(output,readout(rows))
    score.set_defaults(handler=summarize)


def observed_settings(status):
    fields = dict(effective_idle_secs=status.get('index_idle_timeout_secs'),
                  effective_relief=status.get('index_idle_pressure_relief'),
                  client_count=status.get('admitted'))
    return fields, ['unknown_shim_membership'] if status.get('admitted') != 1 else []

def bind_binary(meta):
    result = dict(meta)
    declared = meta.get('binary_sha256')
    actual = hashlib.sha256(Path(meta['binary']).read_bytes()).hexdigest()
    result.update(declared_binary_sha256=declared,binary_sha256=actual)
    reasons = ['missing_binary_pin'] if declared is None else ['binary_pin_mismatch'] if declared != actual else []
    return result,reasons
