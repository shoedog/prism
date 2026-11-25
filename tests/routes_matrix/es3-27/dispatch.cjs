const routes = new Map();

function install(prefix, rows) {
  for (const { method, path, handler } of rows) {
    routes.set(method + ' ' + prefix + path, handler);
  }
}

function dispatch(method, path) {
  return routes.get(method + ' ' + path)?.();
}

module.exports = { dispatch, install };
