import { exec } from 'child_process';
function run(...args) {
  exec(args[0]); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run(t);
}
main();
