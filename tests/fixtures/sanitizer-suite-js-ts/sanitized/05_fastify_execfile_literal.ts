import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("psql", ["-c", arg]);
});

