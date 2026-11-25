import { exec } from 'child_process';
class Handler {
  run(...xs) {
    const x = xs[0];
    exec(x); // @SINK
  }
  entry(other) {
    const t = acquire(); // @SRC
    other.run(t); this.run('fixed');
  }
}
new Handler().entry({ run(x) { observe(x); } });
