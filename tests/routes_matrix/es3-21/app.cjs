const Hapi = require('@hapi/hapi');
const app = Hapi.server();

function recent() { return 'recent'; }
function older() { return 'older'; }
const handlerKey = 'handler';

app.route({ ['method']: 'GET', ['path']: '/recent', ['handler']: recent });
app.route({ method: 'GET', path: '/older', [handlerKey]: older });

module.exports = app;
