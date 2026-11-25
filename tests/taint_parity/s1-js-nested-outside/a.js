import { exec } from 'child_process';
function fixed(x) {
  return 'fixed';
}
function run(x) {
  exec(x); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run(fixed(t));
}
main();
