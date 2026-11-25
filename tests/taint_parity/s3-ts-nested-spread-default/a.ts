import { exec } from 'child_process';
function run(x: string = 'fixed') {
  exec(x); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  observe(run(
    ...([] as []),
    t
  ));
}
