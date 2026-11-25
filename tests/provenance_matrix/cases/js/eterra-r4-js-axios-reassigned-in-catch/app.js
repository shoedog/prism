import axios from 'axios';
async function run(error) {
  try { throw error; } catch (axios) {
    const response = await axios.get('/items');
    console.log(response);
  }
}
