const axios = require('axios');
function edition() {
  return axios.get('https://example.invalid/edition').then(
    ({ data: payload }) => {
      return payload;
    },
    failure => {
      return failure;
    },
  );
}
