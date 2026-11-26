import { exec } from 'child_process';
function run(x) {
  exec(x); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  const emit = run;
  observe(emit(...[], t), run('fixed'));
}
