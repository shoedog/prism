import { readdir as list } from "node:fs";
function f(inp) {
  const t = inp.value; // @SRC
  list(t); // @SINK
}
