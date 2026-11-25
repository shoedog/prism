import { exec } from 'child_process';
function source() {
  const t = acquire(); // @SRC
  return t;
}
function one() {
  const a = source();
  return a;
}
function two() {
  const b = one();
  return b;
}
function main() {
  const result = two();
  exec(result); // @SINK
}
main();
