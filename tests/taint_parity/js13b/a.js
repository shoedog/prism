import { exec } from 'child_process';
function b(y) {
  exec(y); // @SINK
}
function a(x) {
  b(x);
}
function main() {
  const t = getInput(); // @SRC
  a(t);
}
main();
