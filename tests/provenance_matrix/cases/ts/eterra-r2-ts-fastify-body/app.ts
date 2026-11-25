import Fastify from "fastify";
const app = Fastify();
app.post<{ Body: { name: string } }>("/", async (req) => {
  const name = req.body.name;
  console.log(name);
});
