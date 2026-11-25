const { dispatch, install } = require('./dispatch.cjs');

function summary() { return 'summary'; }
const reports = [{ method: 'GET', path: '/summary', handler: summary }];

install('/current', reports);
install('/archive', reports);

module.exports = dispatch;
