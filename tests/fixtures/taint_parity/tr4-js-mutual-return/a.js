import { exec } from 'child_process';
function left(x, n) {
  if (n) return right(x, n - 1);
  return x;
}
function right(x, n) {
  return left(x, n);
}
function main() {
  const t = acquire(); // @SRC
  const y = left(t, 1);
  exec(y); // @SINK
}
main();
