const axios = require("axios");
async function run() {
  const api = axios.create({baseURL: "https://example.org"});
  const response = await api.get("/data");
  const uri = api.getUri({url: "/data"});
  console.log(response, uri);
}
