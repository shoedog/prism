const routes = [];
function route(entries) {
  for (const entry of entries) {
    for (const method of entry.method === "*" ? ["POST", "OPTIONS"] : [entry.method]) {
      routes.push({ ...entry, method });
    }
  }
}
route([
  { method: "*", path: "/mugs", handler: () => "1" },
]);
