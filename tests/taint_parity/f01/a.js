import { exec } from 'child_process';
function d(w) {
  exec(w); // @SINK
}
function c(z) {
  d(z);
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
