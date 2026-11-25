const { install, dispatch } = require('./dispatch');

const reports = { csv() { return 'id,amount'; } };
install([{ method: 'GET', path: '/export', handler: reports.csv }]);

module.exports = { dispatch };
