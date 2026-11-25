import { exec } from 'child_process';
function run(x) {
  exec(x); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  const next = run;
  next?.(t);
}
main();
