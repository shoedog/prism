exports.install = function install(rows) {
  const routes = new Map();
  for (const { method, path, handler } of rows) {
    routes.set(`${method} ${path}`, handler);
  }
  return function dispatch(method, path) {
    const handler = routes.get(`${method} ${path}`);
    return handler ? handler() : null;
  };
};
