import { exec } from 'child_process';
function g(v) {
  exec(v); // @SINK
}
const go = g;
function entry() {
  const t=acquire(); // @SRC
  go(t);
}
