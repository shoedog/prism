function entry(el) {
  const t = acquire(); // @SRC
  const u = acquire();
  const safe = escapeHtml(t);
  const cmd = u;
  el.insertAdjacentHTML('beforeend', safe); exec(cmd); // @SINK
}
import { exec } from 'child_process';
