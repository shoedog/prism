import { exec } from 'child_process';
function leaf(x) { return x; }
function relay(x) {
  const y = leaf(
    x
  );
  consume(y);
}
function consume(x) {
  exec(x); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  relay(t);
}
