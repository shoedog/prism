import { exec } from 'child_process';
function run(value) {
  exec(value); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  run(t); } function other() { run("fixed"); }
