const { install } = require('./dispatch.cjs');

function health() { return 'ready'; }
function count(rows) { return rows.length; }

const routes = [{ method: 'GET', path: '/health', handler: health }];
const routeCount = count(routes);
const app = install(routes);

module.exports = { app, routeCount };
