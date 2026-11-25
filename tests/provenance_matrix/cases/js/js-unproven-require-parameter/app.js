function read(require) {
  const axios = require("axios");
  const value = axios.get("https://example.invalid");
  return value;
}
read(() => ({ get: () => 7 }));
