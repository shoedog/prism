const axios = require("axios");
async function run() {
  const client = axios.create({ baseURL: "https://example.org" });
  const response = await client.get("/items");
  console.log(response);
}
