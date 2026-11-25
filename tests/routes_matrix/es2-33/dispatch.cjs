exports.install = function install(rows) {
  const entries = new Map(rows.map(({ method, path, handler }) => [
    method + ' ' + path, handler,
  ]));
  return function dispatch(method, path) {
    const handler = entries.get(method + ' ' + path);
    return handler ? handler() : null;
  };
};
