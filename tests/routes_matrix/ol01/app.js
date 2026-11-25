const { app, route } = require('./route');

function status(_request, response) { response.end('ready'); }

route([{ method: 'GET', path: '/x', handler: status }]);

module.exports = app;
