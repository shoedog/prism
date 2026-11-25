import { exec } from 'child_process';
function make() {
  const t = acquire(); // @SRC
  return t;
}
function relay(x, count) {
  if (count > 0) return relay(x, count - 1);
  finish(x);
}
function finish(x) {
  last(x);
}
function last(x) {
  exec(x); // @SINK
}
function entry() {
  const x = make();
  relay(x, 1);
}
