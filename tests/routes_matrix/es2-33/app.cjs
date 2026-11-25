const { install } = require('./dispatch.cjs');

function stale() { return 'draft'; }
function published() { return 'published'; }
function health() { return 'ready'; }

const overrides = { handler: published };
const dispatch = install([
  { method: 'GET', path: '/health', handler: health },
  { method: 'GET', path: '/latest', handler: stale, ...overrides },
]);

module.exports = dispatch;
