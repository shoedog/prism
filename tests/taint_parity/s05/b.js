import { exec } from 'child_process';
export function run(v) {
  exec(v); // positive // @SINK
}
