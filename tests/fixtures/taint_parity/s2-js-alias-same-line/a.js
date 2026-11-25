import { exec } from 'child_process';
function run(x) {
  exec(x); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  const go = run;
  run('fixed'); go(t);
}
main();
