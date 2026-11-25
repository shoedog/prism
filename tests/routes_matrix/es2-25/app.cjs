const Hapi = require('@hapi/hapi');
const snapshot = require('./snapshot.cjs');

const server = Hapi.server();
server.route(snapshot);

module.exports = server;
