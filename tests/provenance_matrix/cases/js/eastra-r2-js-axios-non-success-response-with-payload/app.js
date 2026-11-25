const axios = require('axios');
async function lookup() {
  try {
    await axios.get('https://example.invalid/edition');
  } catch (problem) {
    if (!axios.isAxiosError(problem) || !problem.response) return;
    const message = problem.response.data;
    const code = problem.code;
    return [message, code];
  }
}
