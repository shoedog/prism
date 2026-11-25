import { exec } from 'child_process';
class Handler {
  run(p) {
    exec(p.x); // @SINK
  }
  entry(other, key) {
    const t = acquire(); // @SRC
    observe(other[key]({x: t}), this.run({x: 'fixed'}));
  }
}
new Handler().entry({run(p) { observe(p.x); }}, 'run');
