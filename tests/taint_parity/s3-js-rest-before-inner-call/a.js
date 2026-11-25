import { exec } from 'child_process';
function run(...xs) {
  const value = xs[1];
  exec(value); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  observe(run(
    'fixed',
    t
  ));
}
