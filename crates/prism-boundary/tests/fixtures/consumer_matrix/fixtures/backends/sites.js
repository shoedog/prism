import axios from 'axios';

// This page is served from https://reed-vault.invalid and uses both backends.
// The default Axios export has no baseURL override.
const spare = axios.create({ baseURL: 'https://reed-vault.invalid/spare' });
export function list() {
  fetch('/seals'); // GET https://reed-vault.invalid/seals
  axios.get('/seals'); // GET https://reed-vault.invalid/seals
  spare.get('/seals'); // GET https://reed-vault.invalid/spare/seals
  axios.create({ baseURL: 'https://reed-vault.invalid/spare' }).post('/seals', {}); // POST https://reed-vault.invalid/spare/seals
  fetch('https://sable-quay.invalid/seals'); // GET /seals on sable-quay.invalid
}
