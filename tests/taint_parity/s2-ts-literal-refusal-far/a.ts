import { exec } from 'child_process';
declare function acquire(): string;
function tail(x: string) {
  exec(x); // @SINK
}
function run(p: {cmd: string}) {
  tail(p['cmd']);
}
function entry() {
  const t = acquire(); // @SRC
  run({cmd: t});
}
entry();
