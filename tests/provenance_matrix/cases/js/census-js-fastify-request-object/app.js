const fastify = require("fastify"); const app = fastify();
app.get("/preview", (request, reply) => {
  return request;
});
