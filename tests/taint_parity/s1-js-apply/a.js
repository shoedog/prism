import { exec } from 'child_process';
function run(x) {
  exec(x); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run.apply(null, [t]);
}
main();
