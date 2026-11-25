import { exec } from 'child_process';
function main() {
  const t = input(); // @SRC
  exec(t); exec('fixed'); // @SINK
}
