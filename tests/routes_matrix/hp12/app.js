function makeFake() {
    return { route(rows) { return rows.length; } };
}

function status() { return { ready: true }; }

const server = makeFake();
server.route([{ method: 'GET', path: '/x', handler: status }]);

module.exports = server;
