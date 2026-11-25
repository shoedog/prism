import { exec } from 'child_process';
function id(v) {
  return v;
}
function entry() {
  const t = acquire(); // @SRC
  const first = id(
    t
  );
  const second = id(
    t
  );
  exec(second); // @SINK
}
