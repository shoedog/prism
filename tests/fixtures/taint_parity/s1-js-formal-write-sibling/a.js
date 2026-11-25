import { exec } from 'child_process';
function run(p) {
  p.safe = 'other';
  exec(p.cmd); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({cmd: t, safe: 'fixed'});
}
main();
