import { exec } from 'child_process';
function c(z) {
  exec(z); // @SINK
}
function b(y) {
  c(y);
}
function a(x) {
  b(x);
}
function main() {
  const t = getInput(); // @SRC
  a(t);
}
main();
