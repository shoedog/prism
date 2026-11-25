const Hapi = require('@hapi/hapi');
const app = Hapi.server();

function status() { return 'ok'; }
function buildRoute() {
    return { method: 'GET', path: '/health', handler: status };
}

app.route([{ method: 'GET', path: '/ping', handler: status }, buildRoute()]);
module.exports = app;
