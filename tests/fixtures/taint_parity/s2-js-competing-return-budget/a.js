import { exec } from 'child_process';
function send(x) {
  exec(x); // @SINK
}
function identity(x) { return x; }
function entry(flag) {
  const t = acquire(); // @SRC
  if (flag) {
    const copy = identity(
      t);
    send(copy);
  } else {
    send(t);
  }
}
entry(Boolean(choice()));
