import { exec } from 'child_process';
function f(x) {
  return x;
}
function main() {
  const t = getInput(); // @SRC
  exec(f(t)); // @SINK
}
main();
