import { exec } from 'child_process';
function run(p) {
  exec(p.outer.safe); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({outer: {cmd: t, safe: 'fixed'}});
}
main();
