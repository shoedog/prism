import { run } from './b.js';
function entry() {
  const t=acquire(); // @SRC
  run(...[], t);
}
