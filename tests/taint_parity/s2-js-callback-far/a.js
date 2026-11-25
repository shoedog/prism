import { exec } from 'child_process';
function leaf(x) {
  exec(x); // @SINK
}
function handler(value) {
  leaf(value);
}
function entry() {
  const t = acquire(); // @SRC
  const list = [t];
  list.forEach(handler);
}
entry();
