const fastify = require("fastify");
const app = fastify();
app.get("/search", (request, reply) => {
  const term = request.query.term;
  return term;
});
