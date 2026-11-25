import { exec } from 'child_process';
import * as actions from './worker.js';
function main() {
  const t = acquire(); // @SRC
  actions.run(t);
}
main();
