import { exec } from 'child_process';
export default function run(p) {
  exec(p); // @SINK
}
