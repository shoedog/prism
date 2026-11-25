const Hapi = require('@hapi/hapi');
const app = Hapi.server();

async function tenant() { return 'tenant'; }
async function quota() { return 12; }
function audit(request) { return `${request.pre.tenant}:${request.pre.quota}`; }
function summary(request) { return request.pre.audit; }

app.route({
  method: 'GET',
  path: '/summary',
  options: {
    pre: [
      [{ method: tenant, assign: 'tenant' }, { method: quota, assign: 'quota' }],
      { method: audit, assign: 'audit' },
    ],
    handler: summary,
  },
});

module.exports = app;
