import { exec } from 'child_process';
function handler(p) {
  exec(p); // @SINK
}
function main() {
  const t = input(); // @SRC
  const arr = [t];
  arr.forEach(handler); handler(0);
}
