import { exec } from 'child_process';
function run(x) {
  exec(x); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  const go = run;
  observe(go.apply(null, [t]), run('fixed'));
}
