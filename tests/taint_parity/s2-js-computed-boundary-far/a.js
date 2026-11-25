import { exec } from 'child_process';
function leaf(x) {
  exec(x); // @SINK
}
function next(x) {
  leaf(x);
}
function run(p) {
  const k = 'cmd';
  const x = p[k];
  next(x);
}
function main() {
  const t = acquire(); // @SRC
  run({cmd: t});
}
main();
