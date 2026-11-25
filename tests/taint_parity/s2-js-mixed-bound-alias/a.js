import { exec } from 'child_process';
class Runner {
  run(x) {
    exec(x); // @SINK
  }
  check(other) {
    const t = acquire(); // @SRC
    const alias = this;
    other.run(t); alias.run('fixed');
  }
}
new Runner().check({ run(x) { observe(x); } });
