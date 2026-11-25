import { exec } from 'child_process';
class C {
  go() {
    const t = getInput(); // @SRC
    this.run(t);
  }
  run(c) {
    exec(c); // @SINK
  }
}
new C().go();
