import { get as read } from 'config';
function run() {
  const mode = read('mode');
  console.log(mode);
}
