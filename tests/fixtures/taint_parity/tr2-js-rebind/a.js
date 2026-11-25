import { exec } from 'child_process';
function run(x) { exec(x); } // @SINK
function main() {
  let t = acquire(); // @SRC
  run(t); t = 'fixed'; run(t);
}
main();
