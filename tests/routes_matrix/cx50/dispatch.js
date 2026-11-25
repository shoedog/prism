const handlers = new Map();

function install(rows) {
    for (const { method, path, handler } of rows) {
        handlers.set(method + ' ' + path, handler);
    }
}

function dispatch(method, path) {
    return handlers.get(method + ' ' + path)?.();
}

module.exports = { install, dispatch };
