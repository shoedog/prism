import { exec } from 'child_process';
export function run(c) {
  exec(c); // @SINK
}
