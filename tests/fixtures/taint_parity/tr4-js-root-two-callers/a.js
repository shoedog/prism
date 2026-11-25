import { exec } from 'child_process';
function make() {
  const t = acquire(); // @SRC
  return t;
}
function first() {
  const a = make();
  exec(a); // @SINK
}
function second() {
  const b = make();
  observe(b);
}
first();
second();
