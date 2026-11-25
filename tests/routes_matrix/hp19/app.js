const Hapi = require('@hapi/hapi');

const server = Hapi.server();
server.route(require('./routes'));

module.exports = server;
