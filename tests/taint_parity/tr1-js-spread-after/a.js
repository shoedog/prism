import { exec } from 'child_process';
function run(a, b) {
  exec(a); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run(t, ...[]);
}
main();
