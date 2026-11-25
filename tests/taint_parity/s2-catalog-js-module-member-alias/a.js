import * as fileSystem from 'node:fs';
function entry() {
  const t = acquire(); // @SRC
  const list = fileSystem.readdir;
  list(t); // @SINK
}
entry();
