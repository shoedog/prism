function read() {
  require = () => ({ get: () => 7 });
  const axios = require("axios");
  const value = axios.get("https://example.invalid");
  return value;
}
