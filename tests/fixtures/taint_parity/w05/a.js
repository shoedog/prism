import { exec } from 'child_process';
function f(p) {
  const v = p; // @SRC
  return v + '!';
}
function entry(y) {
  const out = f(y);
  exec(out); // @SINK
}
