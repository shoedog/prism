import * as Hapi from '@hapi/hapi';

const app = Hapi.server();
function roster() { return []; }
app.route({ method: 'GET', path: '/roster', vhost: 'staff.example.com', handler: roster });
export default app;
