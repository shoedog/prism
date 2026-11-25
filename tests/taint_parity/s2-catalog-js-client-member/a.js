import axios from 'axios';
function entry() {
  const t = acquire(); // @SRC
  const client = axios.create();
  return client.get(t); // @SINK
}
entry();
