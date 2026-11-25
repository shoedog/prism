import { exec } from 'child_process';
export function run(p) {
  exec(p.cmd); // @SINK
}
