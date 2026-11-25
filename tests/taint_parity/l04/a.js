import { exec } from 'child_process';
function g(p) {
  exec(p); // @SINK
  return p;
}
function main() {
  const t = input(); // @SRC
  g(g(t));
}
