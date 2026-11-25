import Fastify from 'fastify';
import { readdir as list } from 'node:fs';
const app = Fastify();
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
function view(request) {
  const value = request.query.value;
  list(value); // @SINK
}
app.route({ method: 'GET', url: '/view', handler: view });
