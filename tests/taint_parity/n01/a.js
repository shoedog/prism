import { exec } from 'child_process';
function handler(p) {
  exec(p); // @SINK
}
function run(cb, x) {
  cb(x); handler(0);
}
function main() {
  const t = input(); // @SRC
  run(handler, t);
}
