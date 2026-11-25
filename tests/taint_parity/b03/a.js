import { exec } from 'child_process';
function pick(p) {
  exec(p.a); // @SINK
}
function mid(x) {
  pick({ a: x });
}
function outer(x) {
  mid(x);
}
function main() {
  const t = input(); // @SRC
  outer(t);
}
