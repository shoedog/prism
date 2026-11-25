import { exec } from 'child_process';
function keep(x) {
  exec(x); // @SINK
}
function skip(x) { observe(x); }
function main(f) {
  const t = acquire(); // @SRC
  f(t);
}
main(choose() ? keep : skip);
