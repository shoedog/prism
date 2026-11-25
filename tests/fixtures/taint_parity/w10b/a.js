import { exec } from 'child_process';
function a(p) {
  exec(p); // @SINK
  b(p);
}
function b(q) {
  a(q);
}
function main() {
  const t = input(); // @SRC
  a(t);
}
