import { run as go } from './b.js';
import { run as other } from './c.js';
function main() {
  const t = input(); // @SRC
  go(t);
  other('fixed');
}
