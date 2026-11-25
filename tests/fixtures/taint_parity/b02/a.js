import { exec } from 'child_process';
function pick(p) {
  exec(p.a); // @SINK
}
function mid(x) {
  pick({ a: x });
}
function main() {
  const t = input(); // @SRC
  mid(t);
}
