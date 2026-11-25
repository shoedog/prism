import { exec } from 'child_process';
function run(p) {
  p.cmd = 'fixed';
  exec(p.cmd); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({cmd: t});
}
main();
