const fastify = require('fastify')();

function status() { return { ready: true }; }

fastify.register(async (server, opts) => {
    server.route({ method: 'GET', url: '/x', handler: status });
}, { prefix: '/p' });

module.exports = fastify;
