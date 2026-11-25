const name = 'users';

function list() { return []; }

exports.register = function (server, options, next) {
    server.route([{ method: 'GET', path: '/users', handler: list }]);
    next();
};

exports.register.attributes = { name };
