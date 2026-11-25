const { request } = require('node:http');

function receive(response) { response.resume(); }

function fetchStatus() {
    const descriptor = { method: 'GET', path: '/x', handler: receive };
    return request({
        hostname: 'example.test',
        method: descriptor.method,
        path: descriptor.path,
    }, descriptor.handler);
}

module.exports = fetchStatus;
