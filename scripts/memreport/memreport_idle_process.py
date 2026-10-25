"""Normal-serving child and regular-file client transport; no sampling tool traffic."""
import json
import os
import signal
import socket
import subprocess
import time
from pathlib import Path


class Process:
    def __init__(self, command, meta, out, sampler):
        self.meta, self.out, self.sampler = meta, out, sampler
        self.members, self.status, self.owned_daemon = {}, None, None
        self.cleanup_confirmed = meta['mode'] != 'shared'
        self.cleanup_daemons, self.inventory_reasons = {}, []
        self.stdout_path, self.stderr_path = out/'stdout.jsonl', out/'stderr.log'
        self.stdout = self.stdout_path.open('wb', buffering=0)
        self.stderr = self.stderr_path.open('wb', buffering=0)
        env = {k:v for k,v in os.environ.items() if not k.startswith('PRISM_MCP_')}
        if meta['mode'] == 'shared':
            base = Path(meta['runtime_dir'])
            base.mkdir(parents=True, exist_ok=True, mode=0o700)
            runtime = base/'prism-mcpd'
            if runtime.exists() and any(runtime.iterdir()):
                self.stdout.close(); self.stderr.close()
                raise ValueError('fresh private runtime directory required')
            env['PRISM_MCP_RUNTIME_DIR'] = str(base)
        try:
            self.child = subprocess.Popen(command, env=env, stdin=subprocess.PIPE,
                                          stdout=self.stdout, stderr=self.stderr)
        except OSError:
            self.stdout.close(); self.stderr.close()
            raise
        self.reader = self.stdout_path.open('rb')
        self.request_id, self.buffer = 0, b''
        try:
            self.bind(self.child.pid)
        except Exception:
            self.close()
            raise

    def bind(self, pid):
        if pid not in self.members:
            self.members[pid] = self.sampler.read(pid)['os_start']

    def inventory(self):
        # Failure to reverify any required member invalidates this sweep.
        result, self.inventory_reasons = {}, []
        for pid, start in self.members.items():
            value = self.sampler.read(pid,start)
            reason = value.get('reason')
            if not reason and (start is None or value.get('os_start') != start):
                reason = 'missing_os_start' if start is None else 'pid_start_changed'
            if pid == self.child.pid and self.child.poll() is not None:
                reason = reason or 'child_exited'
            if reason:
                self.inventory_reasons.append('inventory_'+str(pid)+': '+reason)
            else:
                result[pid] = start
        return result

    def custody(self):
        if getattr(self,'meta',{}).get('mode') != 'shared':
            return
        runtime = Path(self.meta['runtime_dir'])/'prism-mcpd'
        for path in runtime.glob('*.pid'):
            values = path.read_text().splitlines()
            if not values:
                continue
            pid = int(values[0])
            self.bind(pid)
            self.cleanup_daemons[pid] = self.members[pid]

    def discover(self):
        if self.meta['mode'] != 'shared':
            return [{'path':str(self.stderr_path), 'pid':self.child.pid}]
        self.custody()
        runtime = Path(self.meta['runtime_dir'])/'prism-mcpd'
        files = list(runtime.glob('*.pid'))
        if len(files) != 1:
            return None
        values = files[0].read_text().splitlines()
        if len(values) < 2:
            return None
        pid, start = int(values[0]), int(values[1])
        self.bind(pid)
        sock = files[0].with_suffix('.sock')
        status = self.control(sock, b'STATUS {"control":2}\n')
        if not status.startswith('STATUS '):
            raise ValueError('STATUS unavailable')
        self.status = json.loads(status[7:])
        if self.status.get('pid') != pid or self.status.get('start_time') != start or not self.status.get('identity'):
            raise ValueError('STATUS instance mismatch')
        self.owned_daemon = (sock, dict(self.status))
        instance = dict(pid=pid, instance_start=start*1000000000, identity=self.status['identity'])
        return [{'path':str(self.stderr_path),'require_events':False},
                {'path':str(files[0].with_suffix('.log')), 'instance':instance}]

    @staticmethod
    def control(sock, request):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(2)
            client.connect(str(sock)); client.sendall(request)
            data = b''
            while b'\n' not in data and len(data) < 8192:
                chunk = client.recv(8192-len(data))
                if not chunk:
                    break
                data += chunk
        return data.decode('utf-8')

    def send(self, method, params=None, notify=False):
        self.request_id += 1
        value = dict(jsonrpc='2.0', method=method)
        if not notify:
            value['id'] = self.request_id
        if params is not None:
            value['params'] = params
        self.child.stdin.write((json.dumps(value)+'\n').encode())
        self.child.stdin.flush()
        return value.get('id')

    def responses(self):
        self.buffer += self.reader.read()
        lines = self.buffer.split(b'\n')
        self.buffer = lines.pop()
        return [(json.loads(line), time.time_ns()) for line in lines if line]

    def same_identity(self, pid, start):
        value = self.sampler.read(pid,start)
        gone = value.get('reason') in ('rusage_ESRCH','exited_process','pid_start_changed')
        same = start is not None and value.get('os_start') == start and not value.get('reason')
        return same, gone

    def close(self):
        self.cleanup_confirmed = False
        self.cleanup_daemons = getattr(self,'cleanup_daemons',{})
        custody_ok = True
        try:
            self.custody()
        except Exception:
            custody_ok = False
        try:
            self.child.stdin.close()
        except (BrokenPipeError, OSError):
            pass
        child_done = False
        try:
            self.child.wait(timeout=5)
            child_done = True
        except subprocess.TimeoutExpired:
            same, gone = self.same_identity(self.child.pid,self.members.get(self.child.pid))
            if same:
                self.child.kill(); self.child.wait(timeout=5)
            child_done = same or gone
        try:
            self.custody()  # launch may have finished while initialize/EOF failed
        except Exception:
            custody_ok = False
        if self.owned_daemon:
            sock, status = self.owned_daemon
            self.cleanup_daemons[status['pid']] = self.members.get(status['pid'])
        all_gone = True
        for pid, start in self.cleanup_daemons.items():
            same, gone = self.same_identity(pid,start)
            if not same:
                all_gone &= gone
                continue
            try:
                if self.owned_daemon and self.owned_daemon[1]['pid']==pid:
                    sock,status = self.owned_daemon
                    payload = dict(control=2,**{k:status[k] for k in ('identity','pid','start_time')})
                    self.control(sock,('SHUTDOWN '+json.dumps(payload,separators=(',',':'))+'\n').encode())
                else:
                    os.kill(pid,signal.SIGTERM)  # fresh private runtime + verified OS start
                until = time.monotonic()+5
                while time.monotonic() < until:
                    same,gone = self.same_identity(pid,start)
                    if gone:
                        break
                    time.sleep(.025)
            except OSError:
                _,gone = self.same_identity(pid,start)
            all_gone &= gone
        shared = getattr(self,'meta',{}).get('mode')=='shared' or bool(self.owned_daemon)
        self.cleanup_confirmed = bool(child_done and custody_ok and all_gone and
                                      (not shared or self.cleanup_daemons))
        self.reader.close(); self.stdout.close(); self.stderr.close()
