import { exec } from 'child_process';
function run(x: string, ...rest: string[]) {
  exec(x); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  const alias = run;
  observe(alias?.(t, 'fixed'));
}
