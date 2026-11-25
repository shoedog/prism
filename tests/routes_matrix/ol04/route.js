const { createServer } = require('node:http');

const handlers = new Map();
const app = createServer((request, response) => {
    const path = new URL(request.url, 'http://localhost').pathname;
    const handler = handlers.get(`${request.method} ${path}`);
    if (handler) {
        return handler(request, response);
    }
    response.writeHead(404).end();
});

function route(rows) {
    for (const { method, path, handler } of rows) {
        for (const verb of [].concat(method)) {
            handlers.set(`${verb} ${path}`, handler);
        }
    }
}

module.exports = { app, route };
