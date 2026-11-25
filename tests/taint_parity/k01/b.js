import { exec } from 'child_process';
export class W {
  run(c) {
    exec(c); // @SINK
  }
}
