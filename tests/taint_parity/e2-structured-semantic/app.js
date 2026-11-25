import yaml from "js-yaml";
function f(inp) {
  const t = inp.value; // @SRC
  yaml.load(t); // @SINK
}
