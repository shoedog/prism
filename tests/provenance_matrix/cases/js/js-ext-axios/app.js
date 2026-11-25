const axios = require("axios");

async function read(url) {
  const response = await axios.get(url);
  return response;
}
