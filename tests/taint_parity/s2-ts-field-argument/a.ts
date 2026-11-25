import { exec } from 'child_process';
declare function acquire(): string;
function run(x: string) {
  exec(x); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  const p = {cmd: 'fixed'};
  p.cmd = t;
  run(p.cmd);
}
entry();
