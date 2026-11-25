function status(_request, response) { response.end('ready'); }

exports.routes = [{ method: 'GET', path: '/x', handler: status }];
