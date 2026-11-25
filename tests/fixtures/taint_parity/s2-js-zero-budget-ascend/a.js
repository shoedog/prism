import { exec } from 'child_process';
function source() {
  const t = acquire(); // @SRC
  return t;
}
function main() {
  const result = source();
  exec(result); // @SINK
}
main();
