function status(_request, response) { response.end('ready'); }

function route(rows) { return rows.length; }

module.exports = route([{ method: 'GET', path: '/x', handler: status }]);
