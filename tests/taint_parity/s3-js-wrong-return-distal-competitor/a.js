import { exec } from 'child_process';
function echo(x) { return x; }
function relay(x) { finish(x); }
function finish(x) {
  exec(x); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  echo(
    t
  );
}
function other() {
  const y = echo('fixed');
  relay(y);
}
