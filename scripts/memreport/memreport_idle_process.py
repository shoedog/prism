"""Normal-serving child and regular-file client transport; no sampling tool traffic."""
import json
import os
import socket
import subprocess
import time
from pathlib import Path


class Process:
    def __init__(self, command, meta, out, sampler):
        self.meta, self.out, self.sampler = meta, out, sampler
        self.members, self.status, self.owned_daemon = {}, None, None
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
        self.bind(self.child.pid)
        self.reader = self.stdout_path.open('rb')
        self.request_id, self.buffer = 0, b''

    def bind(self, pid):
        if pid not in self.members:
            self.members[pid] = self.sampler.read(pid)['os_start']

    def inventory(self):
        # Every launched/discovered member remains required. A missing one is not a saving.
        return {pid:start for pid,start in self.members.items()
                if self.sampler.read(pid, start)['reason'] != 'exited_process' and
                not (pid == self.child.pid and self.child.poll() is not None)}

    def discover(self):
        if self.meta['mode'] != 'shared':
            return [{'path':str(self.stderr_path), 'pid':self.child.pid}]
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
        if self.status['pid'] != pid or self.status['start_time'] != start:
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

    def close(self):
        self.cleanup_confirmed = not self.owned_daemon
        try:
            self.child.stdin.close()
        except BrokenPipeError:
            pass
        try:
            self.child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.child.kill(); self.child.wait()
        if self.owned_daemon:
            sock, status = self.owned_daemon
            payload = {k:status[k] for k in ('identity','pid','start_time')}
            payload = dict(control=2, **payload)
            try:
                self.control(sock, ('SHUTDOWN '+json.dumps(payload,separators=(',',':'))+'\n').encode())
                until = time.monotonic()+5
                while time.monotonic() < until:
                    final = self.sampler.read(status['pid'], self.members.get(status['pid']))
                    if final['reason'] in ('rusage_ESRCH','exited_process','pid_start_changed'):
                        self.cleanup_confirmed = True
                        break
                    time.sleep(.025)
            except OSError:
                pass  # raw failure is retained; caller excludes an unconfirmed shutdown
        self.reader.close(); self.stdout.close(); self.stderr.close()
