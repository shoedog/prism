function fetchConfig(name) { return name.startsWith("show"); }
function show() {
  const state = fetchConfig("show-mode");
  return state;
}
