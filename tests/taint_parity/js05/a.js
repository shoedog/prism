import { exec } from 'child_process';
function f(x) {
  return x;
}
function main() {
  const t = getInput(); // @SRC
  const y = f(t);
  exec(y); // @SINK
}
main();
