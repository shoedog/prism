import { get as sendRequest } from "axios";
function f(inp) {
  const t = inp.value; // @SRC
  sendRequest(t); // @SINK
}
