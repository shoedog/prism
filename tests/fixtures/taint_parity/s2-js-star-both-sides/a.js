import { exec } from 'child_process';
function run(a, b) {
  exec(b); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run(...[], t, ...['fixed']);
}
main();
