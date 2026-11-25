const routes = [];
function route(entries) { routes.push(...entries); }
route([
  { method: "POST", path: "/mugs", finish: "ash", handler: () => "1" },
  { method: "POST", path: "/mugs", finish: "salt", handler: () => "2" },
]);
function dispatch(method, path, headers) {
  const match = routes.find((entry) => entry.method === method && entry.path === path &&
    entry.finish === headers["x-finish"]);
  return match?.handler();
}
