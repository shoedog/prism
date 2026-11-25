import { exec } from 'child_process';
function make() {
  const t = acquire(); // @SRC
  return t;
}
function finish(x) {
  exec(x); // @SINK
}
function forward(x) {
  finish(x);
}
function main() {
  const out = make();
  forward(out);
}
main();
