import { exec } from 'child_process';
function run(p) {
  const key = 'cmd';
  exec(p[key]); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({cmd: t});
}
main();
