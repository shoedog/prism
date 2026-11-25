import { run as go } from './b.js';
function main() {
  const t = acquire(); // @SRC
  go({cmd: t});
}
main();
