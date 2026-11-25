import { exec } from 'child_process';
function id(v) { return v; }
function send(v) {
  exec(v); // @SINK
}
function entry(flag) {
  const t = acquire(); // @SRC
  let joined;
  if (flag) {
    joined = id(
      t
    );
  } else {
    const a = t;
    const b = a;
    const c = b;
    const d = c;
    joined = d;
  }
  send(joined);
}
