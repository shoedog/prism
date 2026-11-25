import { exec } from 'child_process';
export function one(x) {
  exec(x); // @SINK
}
