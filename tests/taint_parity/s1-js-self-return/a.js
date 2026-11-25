import { exec } from 'child_process';
function run(n) {
  if (n === 0) {
    const t = acquire(); // @SRC
    return t;
  }
  const y = run(0);
  exec(y); // @SINK
  return y;
}
run(1);
