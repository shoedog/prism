import fastify from "fastify";
import { exec } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return exec(`psql -c ${arg}`);
});

