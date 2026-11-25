const Hapi = require('@hapi/hapi');
const server = Hapi.server();

function overview() { return { dashboards: [] }; }
function configuration(currentServer) {
  return { handler: overview };
}

server.route({ method: 'GET', path: '/overview', options: configuration });

module.exports = server;
