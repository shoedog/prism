import { exec } from 'child_process';
function run(p) {
  exec(p.outer.cmd); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({outer: t});
}
main();
