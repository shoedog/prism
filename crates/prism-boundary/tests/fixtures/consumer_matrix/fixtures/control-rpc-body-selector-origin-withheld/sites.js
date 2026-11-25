export function statusAndInventory() {
  fetch('/dispatch'); // GET /dispatch
  fetch('/dispatch?action=inventory'); // GET /dispatch, query-selected inventory
}
export function receiptsAndBouquets() {
  fetch('/query', { method: 'POST', body: '{}' }); // POST /query
  fetch('/query', { method: 'POST', body: JSON.stringify({ query: '{ bouquets { shade } }' }) }); // POST /query, body-selected bouquets
}
